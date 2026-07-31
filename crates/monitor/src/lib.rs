#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use luna_assembler::{assemble, assemble_program as assemble_source_program};
use luna_diag::{Diagnostic, Result};
use luna_disassembler::disassemble_word;
use luna_isa::Instruction;
use luna_machine::{FFLAG_DZ, FFLAG_NV, FFLAG_NX, FFLAG_OF, FFLAG_UF, Machine};
use luna_target_api::{ExecutionOutcome, MemoryAccess, MemoryAccessKind, StopEvent, TargetBackend};

const DEFAULT_MEMORY_VIEW_BYTES: usize = 64;
const MAX_MEMORY_VIEW_BYTES: usize = 4096;
const MAX_EDIT_BYTES: usize = 4096;
const MAX_UNDO_ENTRIES: usize = 64;
const MAX_HISTORY_ENTRIES: usize = 4096;
const MAX_PERSISTED_BYTES: usize = 64 * 1024 * 1024;
const SNAPSHOT_MAGIC: &[u8; 8] = b"RVSNAP01";
const PROJECT_MAGIC: &[u8; 8] = b"RVPROJ01";
const PERSISTENCE_VERSION: u32 = 1;

struct MemoryEdit {
    address: u64,
    previous: Vec<u8>,
}

struct BackendBreakpoint {
    id: u64,
    address: u64,
}

#[derive(Clone, Copy)]
struct Watchpoint {
    address: u64,
    width: u64,
    kind: Option<MemoryAccessKind>,
    id: u64,
}

struct HistoryEntry {
    sequence: u64,
    pc_before: u64,
    pc_after: u64,
    instruction: String,
    memory_access: Option<MemoryAccess>,
}

struct CallFrame {
    return_pc: u64,
    target: u64,
}

struct DecodedSnapshot {
    machine: Machine,
    symbols: BTreeMap<u64, String>,
    marks: BTreeMap<String, u64>,
    breakpoints: BTreeMap<u64, u64>,
    watchpoints: BTreeMap<u64, Watchpoint>,
    next_breakpoint_id: u64,
    next_watchpoint_id: u64,
    view_address: u64,
}

pub struct Monitor {
    pub machine: Machine,
    symbols: BTreeMap<u64, String>,
    max_run_steps: u64,
    view_address: u64,
    undo: Vec<MemoryEdit>,
    marks: BTreeMap<String, u64>,
    breakpoints: BTreeMap<u64, u64>,
    watchpoints: BTreeMap<u64, Watchpoint>,
    next_breakpoint_id: u64,
    next_watchpoint_id: u64,
    history: Vec<HistoryEntry>,
    call_stack: Vec<CallFrame>,
    source_text: String,
}

impl Monitor {
    pub fn new(memory_size: usize) -> Self {
        Self {
            machine: Machine::new(memory_size),
            symbols: BTreeMap::new(),
            max_run_steps: 1000,
            view_address: 0,
            undo: Vec::new(),
            marks: BTreeMap::new(),
            breakpoints: BTreeMap::new(),
            watchpoints: BTreeMap::new(),
            next_breakpoint_id: 1,
            next_watchpoint_id: 1,
            history: Vec::new(),
            call_stack: Vec::new(),
            source_text: String::new(),
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
            "assemble-program" | "load" => self.assemble_program(argument),
            "step" | "s" => self.step(),
            "run" | "r" => self.run(argument),
            "continue" | "c" => self.continue_target(argument),
            "disasm" | "d" => self.disassemble(argument),
            "memory" | "mem" | "hex" | "ascii" => self.memory_view(argument),
            "view" | "jump" => self.set_view(argument),
            "edit" | "e" => self.edit_memory(argument),
            "undo" | "u" => self.undo_memory(),
            "mark" => self.mark(argument),
            "marks" => self.list_marks(),
            "unmark" => self.unmark(argument),
            "break" | "b" => self.add_breakpoint(argument),
            "watch" => self.add_watchpoint(argument, Some(MemoryAccessKind::Write)),
            "rwatch" => self.add_watchpoint(argument, Some(MemoryAccessKind::Read)),
            "awatch" => self.add_watchpoint(argument, None),
            "delete" | "del" => self.delete_debug_item(argument),
            "info" => self.info_debug(argument),
            "history" | "trace" => self.show_history(argument),
            "stack" | "bt" => self.show_stack(),
            "where" => self.show_location(),
            "symbols" => self.show_symbols(),
            "snapshot" => self.snapshot_file(argument),
            "restore" => self.restore_snapshot_file(argument),
            "project-save" => self.project_save_file(argument),
            "project-load" => self.project_load_file(argument),
            "reset" => {
                self.machine = Machine::new(self.machine.memory_size());
                self.symbols.clear();
                self.view_address = 0;
                self.undo.clear();
                self.marks.clear();
                self.breakpoints.clear();
                self.watchpoints.clear();
                self.history.clear();
                self.call_stack.clear();
                self.source_text.clear();
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
        self.view_address = address;
        self.source_text.clear();
        let word = image
            .text
            .get(..4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("four-byte instruction")));
        Ok(match word {
            Some(word) => format!("loaded 0x{word:08x} at 0x{address:016x}"),
            None => format!("loaded {} bytes at 0x{address:016x}", image.text.len()),
        })
    }

    pub fn assemble_program(&mut self, source: &str) -> Result<String> {
        if source.trim().is_empty() {
            return Err(Diagnostic::error(
                "MON-ASM-002",
                "assemble-program expects at least one source line",
            ));
        }
        let image = assemble_source_program(source)?;
        self.machine = Machine::new(self.machine.memory_size());
        self.machine.load(image.entry, &image.text)?;
        self.machine.pc = image.entry;
        self.symbols = image
            .symbols
            .into_iter()
            .map(|(name, address)| (address + image.entry, name))
            .collect();
        self.view_address = image.entry;
        self.undo.clear();
        self.marks.clear();
        self.breakpoints.clear();
        self.watchpoints.clear();
        self.history.clear();
        self.call_stack.clear();
        self.source_text = source.to_string();
        Ok(format!(
            "loaded {} bytes at 0x{:016x}; {} symbol(s)",
            image.text.len(),
            image.entry,
            self.symbols.len()
        ))
    }

    pub fn snapshot_bytes(&mut self) -> Result<Vec<u8>> {
        let mut memory = vec![0u8; self.machine.memory_size()];
        TargetBackend::read_memory(&mut self.machine, 0, &mut memory)?;
        let mut writer = ByteWriter::new(SNAPSHOT_MAGIC);
        writer.u32(PERSISTENCE_VERSION);
        writer.u64(memory.len() as u64);
        for value in self.machine.x {
            writer.u64(value);
        }
        for value in self.machine.f {
            writer.u64(value);
        }
        writer.u32(self.machine.fcsr);
        writer.u64(self.machine.pc);
        writer.u64(self.machine.instructions);
        writer.u64(self.view_address);
        writer.u64(self.next_breakpoint_id);
        writer.u64(self.next_watchpoint_id);
        writer.map_u64_string(&self.symbols)?;
        writer.map_string_u64(&self.marks)?;
        writer.u32(self.breakpoints.len() as u32);
        for (address, id) in &self.breakpoints {
            writer.u64(*address);
            writer.u64(*id);
        }
        writer.u32(self.watchpoints.len() as u32);
        for watchpoint in self.watchpoints.values() {
            writer.u64(watchpoint.id);
            writer.u64(watchpoint.address);
            writer.u64(watchpoint.width);
            writer.u8(match watchpoint.kind {
                None => 0,
                Some(MemoryAccessKind::Read) => 1,
                Some(MemoryAccessKind::Write) => 2,
            });
        }
        writer.bytes(&memory);
        let bytes = writer.finish();
        if bytes.len() > MAX_PERSISTED_BYTES {
            return Err(Diagnostic::error(
                "MON-PERSIST-032",
                "snapshot exceeds size limit",
            ));
        }
        Ok(bytes)
    }

    pub fn restore_snapshot_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let decoded = decode_snapshot(bytes)?;
        self.apply_snapshot(decoded);
        Ok(())
    }

    fn apply_snapshot(&mut self, decoded: DecodedSnapshot) {
        self.machine = decoded.machine;
        self.symbols = decoded.symbols;
        self.marks = decoded.marks;
        self.breakpoints = decoded.breakpoints;
        self.watchpoints = decoded.watchpoints;
        self.next_breakpoint_id = decoded.next_breakpoint_id;
        self.next_watchpoint_id = decoded.next_watchpoint_id;
        self.view_address = decoded.view_address;
        self.undo.clear();
        self.history.clear();
        self.call_stack.clear();
    }

    fn snapshot_file(&mut self, argument: &str) -> Result<String> {
        let path = persistence_path(argument, "MON-PERSIST-001")?;
        let bytes = self.snapshot_bytes()?;
        fs::write(path, &bytes).map_err(|error| {
            Diagnostic::error("MON-PERSIST-002", format!("cannot write snapshot: {error}"))
        })?;
        Ok(format!("snapshot saved ({} bytes)", bytes.len()))
    }

    fn restore_snapshot_file(&mut self, argument: &str) -> Result<String> {
        let path = persistence_path(argument, "MON-PERSIST-003")?;
        let bytes = read_persistence_file(path)?;
        self.restore_snapshot_bytes(&bytes)?;
        Ok(format!("snapshot restored ({} bytes)", bytes.len()))
    }

    fn project_save_file(&mut self, argument: &str) -> Result<String> {
        let path = persistence_path(argument, "MON-PERSIST-004")?;
        let snapshot = self.snapshot_bytes()?;
        let mut writer = ByteWriter::new(PROJECT_MAGIC);
        writer.u32(PERSISTENCE_VERSION);
        writer.string(&self.source_text)?;
        writer.u64(snapshot.len() as u64);
        writer.bytes(&snapshot);
        let bytes = writer.finish();
        if bytes.len() > MAX_PERSISTED_BYTES {
            return Err(Diagnostic::error(
                "MON-PERSIST-033",
                "project exceeds size limit",
            ));
        }
        fs::write(path, &bytes).map_err(|error| {
            Diagnostic::error("MON-PERSIST-005", format!("cannot write project: {error}"))
        })?;
        Ok(format!("project saved ({} bytes)", bytes.len()))
    }

    fn project_load_file(&mut self, argument: &str) -> Result<String> {
        let path = persistence_path(argument, "MON-PERSIST-006")?;
        let bytes = read_persistence_file(path)?;
        let (source, snapshot) = decode_project(&bytes)?;
        self.restore_snapshot_bytes(&snapshot)?;
        self.source_text = source;
        Ok(format!("project restored ({} bytes)", bytes.len()))
    }

    fn step(&mut self) -> Result<String> {
        let address = self.machine.pc;
        let word = self.read_word(address)?;
        let line = disassemble_word(address, word, &self.symbols);
        let outcome = TargetBackend::step(&mut self.machine)?;
        let pc_after = match outcome {
            ExecutionOutcome::Retired {
                pc_before,
                pc_after,
                memory_access,
            } => {
                self.record_retired(
                    pc_before,
                    pc_after,
                    line.instruction,
                    &line.text,
                    memory_access,
                );
                pc_after
            }
            ExecutionOutcome::Stopped(event) => event.pc,
            ExecutionOutcome::BudgetExhausted { pc, .. } => pc,
        };
        Ok(format!(
            "0x{address:016x}: {word:08x}  {:<28} -> pc=0x{pc_after:016x}",
            line.text
        ))
    }

    fn run(&mut self, argument: &str) -> Result<String> {
        let limit = parse_run_limit(argument, self.max_run_steps)?;
        self.run_with_limit(limit, false)
    }

    fn continue_target(&mut self, argument: &str) -> Result<String> {
        let limit = parse_run_limit(argument, self.max_run_steps)?;
        self.run_with_limit(limit, true)
    }

    fn run_with_limit(&mut self, limit: u64, bypass_current_breakpoint: bool) -> Result<String> {
        let start = self.machine.instructions;
        let mut bypass =
            bypass_current_breakpoint && self.breakpoints.contains_key(&self.machine.pc);
        for _ in 0..limit {
            if !bypass {
                if let Some(id) = self.breakpoints.get(&self.machine.pc) {
                    return Ok(format!(
                        "stopped: breakpoint #{id} at pc=0x{:016x}; total={}",
                        self.machine.pc, self.machine.instructions
                    ));
                }
            }
            let pc_before = self.machine.pc;
            let word = self.read_word(pc_before)?;
            let line = disassemble_word(pc_before, word, &self.symbols);
            let outcome = TargetBackend::step(&mut self.machine)?;
            match outcome {
                ExecutionOutcome::Retired {
                    pc_after,
                    memory_access,
                    ..
                } => {
                    bypass = false;
                    self.record_retired(
                        pc_before,
                        pc_after,
                        line.instruction,
                        &line.text,
                        memory_access,
                    );
                    if let Some(access) = memory_access {
                        if let Some(watchpoint) = self.watchpoint_hit(access) {
                            return Ok(format_watchpoint_stop(
                                watchpoint,
                                access,
                                self.machine.pc,
                                self.machine.instructions,
                            ));
                        }
                    }
                    if self.machine.pc != pc_after {
                        return Err(Diagnostic::error(
                            "MON-DEBUG-001",
                            "backend returned an inconsistent program counter",
                        ));
                    }
                }
                ExecutionOutcome::Stopped(event) => {
                    return Ok(format_backend_stop(event));
                }
                ExecutionOutcome::BudgetExhausted { .. } => {
                    return Err(Diagnostic::error(
                        "MON-DEBUG-002",
                        "step backend returned a run-only outcome",
                    ));
                }
            }
        }
        Ok(format!(
            "ran {} step(s); pc=0x{:016x}; total={}",
            self.machine.instructions - start,
            self.machine.pc,
            self.machine.instructions
        ))
    }

    fn record_retired(
        &mut self,
        pc_before: u64,
        pc_after: u64,
        instruction: Instruction,
        text: &str,
        memory_access: Option<MemoryAccess>,
    ) {
        if self.history.len() == MAX_HISTORY_ENTRIES {
            self.history.remove(0);
        }
        self.history.push(HistoryEntry {
            sequence: self.machine.instructions,
            pc_before,
            pc_after,
            instruction: text.to_string(),
            memory_access,
        });
        match instruction {
            Instruction::Jal(luna_isa::Jal { rd: 1, imm }) => {
                self.call_stack.push(CallFrame {
                    return_pc: pc_before.wrapping_add(4),
                    target: pc_before.wrapping_add_signed(i64::from(imm)),
                });
            }
            Instruction::Jalr(luna_isa::Jalr { rd: 0, rs1: 1, .. }) => {
                self.call_stack.pop();
            }
            _ => {}
        }
    }

    fn show_history(&self, argument: &str) -> Result<String> {
        let requested = if argument.trim().is_empty() {
            16
        } else {
            parse_count(argument.trim(), "MON-HIST-001")?
        };
        let count = requested.min(256).min(self.history.len());
        let start = self.history.len() - count;
        if count == 0 {
            return Ok("history: empty".into());
        }
        let mut output = String::from("history:\n");
        for entry in &self.history[start..] {
            write!(
                output,
                "  #{:06}  0x{:016x} -> 0x{:016x}  {}",
                entry.sequence, entry.pc_before, entry.pc_after, entry.instruction
            )
            .unwrap();
            if let Some(access) = entry.memory_access {
                let kind = match access.kind {
                    MemoryAccessKind::Read => "read",
                    MemoryAccessKind::Write => "write",
                };
                write!(
                    output,
                    " [{kind} 0x{:016x}/{}]",
                    access.address, access.width
                )
                .unwrap();
            }
            output.push('\n');
        }
        Ok(output.trim_end().into())
    }

    fn show_stack(&self) -> Result<String> {
        if self.call_stack.is_empty() {
            return Ok("stack: empty".into());
        }
        let mut output = String::from("stack:\n");
        for (depth, frame) in self.call_stack.iter().rev().enumerate() {
            writeln!(
                output,
                "  #{depth} target={} return=0x{:016x}",
                self.format_symbol(frame.target),
                frame.return_pc
            )
            .unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn show_location(&self) -> Result<String> {
        Ok(format!(
            "pc=0x{:016x} {} view=0x{:016x}",
            self.machine.pc,
            self.format_symbol(self.machine.pc),
            self.view_address
        ))
    }

    fn show_symbols(&self) -> Result<String> {
        if self.symbols.is_empty() {
            return Ok("symbols: none".into());
        }
        let mut output = String::from("symbols:\n");
        for (address, name) in &self.symbols {
            writeln!(output, "  0x{address:016x} {name}").unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn format_symbol(&self, address: u64) -> String {
        self.symbols
            .range(..=address)
            .next_back()
            .map(|(symbol_address, name)| {
                let offset = address - symbol_address;
                if offset == 0 {
                    name.clone()
                } else {
                    format!("{name}+0x{offset:x}")
                }
            })
            .unwrap_or_else(|| "<no-symbol>".into())
    }

    fn disassemble(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (address, count) = match parts.as_slice() {
            [] => (self.machine.pc, 4),
            [count] => (self.machine.pc, parse_count(count, "MON-DISASM-001")?),
            [address, count] => (
                self.resolve_address(address, "MON-DISASM-002")?,
                parse_count(count, "MON-DISASM-001")?,
            ),
            _ => {
                return Err(Diagnostic::error(
                    "MON-DISASM-001",
                    "disasm expects [count] or [address] [count]",
                ));
            }
        };
        let mut output = String::new();
        for index in 0..count {
            let offset = u64::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(4))
                .ok_or_else(|| Diagnostic::error("MON-DISASM-002", "address overflow"))?;
            let address = address
                .checked_add(offset)
                .ok_or_else(|| Diagnostic::error("MON-DISASM-002", "address overflow"))?;
            let word = self.read_word(address)?;
            let line = disassemble_word(address, word, &self.symbols);
            writeln!(output, "0x{address:016x}: {word:08x}  {}", line.text).unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn read_word(&mut self, address: u64) -> Result<u32> {
        let mut bytes = [0u8; 4];
        TargetBackend::read_memory(&mut self.machine, address, &mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn set_view(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let [address] = parts.as_slice() else {
            return Err(Diagnostic::error(
                "MON-MEM-001",
                "view expects one target address",
            ));
        };
        self.view_address = self.resolve_address(address, "MON-MEM-002")?;
        Ok(format!("view=0x{:016x}", self.view_address))
    }

    fn memory_view(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (address, count) = match parts.as_slice() {
            [] => (self.view_address, DEFAULT_MEMORY_VIEW_BYTES),
            [address] => (
                self.resolve_address(address, "MON-MEM-002")?,
                DEFAULT_MEMORY_VIEW_BYTES,
            ),
            [address, count] => (
                self.resolve_address(address, "MON-MEM-002")?,
                parse_count(count, "MON-MEM-003")?,
            ),
            _ => {
                return Err(Diagnostic::error(
                    "MON-MEM-001",
                    "memory expects [address] [count]",
                ));
            }
        };
        if count > MAX_MEMORY_VIEW_BYTES {
            return Err(Diagnostic::error(
                "MON-MEM-004",
                "memory view exceeds the 4096-byte interactive limit",
            ));
        }
        let mut bytes = vec![0u8; count];
        TargetBackend::read_memory(&mut self.machine, address, &mut bytes)?;
        self.view_address = address;

        let mut output = String::new();
        for (row, chunk) in bytes.chunks(16).enumerate() {
            let row_address = address
                .checked_add((row * 16) as u64)
                .ok_or_else(|| Diagnostic::error("MON-MEM-002", "address overflow"))?;
            write!(output, "0x{row_address:016x}: ").unwrap();
            for byte in chunk {
                write!(output, "{byte:02x} ").unwrap();
            }
            for _ in chunk.len()..16 {
                output.push_str("   ");
            }
            output.push_str("|");
            for byte in chunk {
                output.push(if (0x20..=0x7e).contains(byte) {
                    *byte as char
                } else {
                    '.'
                });
            }
            output.push('|');
            output.push('\n');
        }
        Ok(output.trim_end().into())
    }

    fn edit_memory(&mut self, argument: &str) -> Result<String> {
        let mut parts = argument.split_whitespace();
        let address_text = parts.next().ok_or_else(|| {
            Diagnostic::error("MON-MEM-005", "edit expects an address followed by bytes")
        })?;
        let address = self.resolve_address(address_text, "MON-MEM-002")?;
        let mut bytes = Vec::new();
        for token in parts {
            parse_byte_token(token, &mut bytes)?;
            if bytes.len() > MAX_EDIT_BYTES {
                return Err(Diagnostic::error(
                    "MON-MEM-006",
                    "edit exceeds the 4096-byte transaction limit",
                ));
            }
        }
        if bytes.is_empty() {
            return Err(Diagnostic::error(
                "MON-MEM-005",
                "edit expects at least one byte",
            ));
        }
        let mut previous = vec![0u8; bytes.len()];
        TargetBackend::read_memory(&mut self.machine, address, &mut previous)?;
        TargetBackend::write_memory(&mut self.machine, address, &bytes)?;
        if self.undo.len() == MAX_UNDO_ENTRIES {
            self.undo.remove(0);
        }
        self.undo.push(MemoryEdit { address, previous });
        self.view_address = address;
        Ok(format!(
            "edited {} byte(s) at 0x{address:016x}",
            bytes.len()
        ))
    }

    fn undo_memory(&mut self) -> Result<String> {
        let (address, previous) = self
            .undo
            .last()
            .map(|edit| (edit.address, edit.previous.clone()))
            .ok_or_else(|| Diagnostic::error("MON-MEM-007", "nothing to undo"))?;
        TargetBackend::write_memory(&mut self.machine, address, &previous)?;
        self.undo.pop();
        self.view_address = address;
        Ok(format!(
            "undid {} byte(s) at 0x{address:016x}",
            previous.len()
        ))
    }

    fn mark(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (name, address) = match parts.as_slice() {
            [name] => (*name, self.view_address),
            [name, address] => (*name, self.resolve_address(address, "MON-MARK-002")?),
            _ => {
                return Err(Diagnostic::error(
                    "MON-MARK-001",
                    "mark expects <name> [address]",
                ));
            }
        };
        validate_mark_name(name)?;
        self.marks.insert(name.to_string(), address);
        Ok(format!("mark @{name}=0x{address:016x}"))
    }

    fn list_marks(&self) -> Result<String> {
        if self.marks.is_empty() {
            return Ok("marks: none".into());
        }
        let mut output = String::from("marks:\n");
        for (name, address) in &self.marks {
            writeln!(output, "  @{name}=0x{address:016x}").unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn unmark(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let [name] = parts.as_slice() else {
            return Err(Diagnostic::error("MON-MARK-003", "unmark expects one name"));
        };
        let name = name.strip_prefix('@').unwrap_or(name);
        if self.marks.remove(name).is_none() {
            return Err(Diagnostic::error("MON-MARK-004", "mark does not exist"));
        }
        Ok(format!("unmarked @{name}"))
    }

    fn resolve_address(&self, value: &str, code: &'static str) -> Result<u64> {
        if let Some(name) = value.strip_prefix('@') {
            if name.is_empty() {
                return Err(Diagnostic::error(code, "mark name is empty"));
            }
            return self.marks.get(name).copied().ok_or_else(|| {
                Diagnostic::error("MON-MARK-005", format!("unknown mark: @{name}"))
            });
        }
        parse_address(value, code)
    }

    fn add_breakpoint(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let [address_text] = parts.as_slice() else {
            return Err(Diagnostic::error(
                "MON-DBG-001",
                "break expects one address",
            ));
        };
        let address = self.resolve_address(address_text, "MON-DBG-002")?;
        if address % 4 != 0 {
            return Err(Diagnostic::error(
                "MON-DBG-003",
                "breakpoint address must be 4-byte aligned for the current profile",
            ));
        }
        if let Some(id) = self.breakpoints.get(&address) {
            return Ok(format!("breakpoint #{id} already enabled"));
        }
        let id = self.next_breakpoint_id;
        self.next_breakpoint_id = id
            .checked_add(1)
            .ok_or_else(|| Diagnostic::error("MON-DBG-004", "breakpoint id exhausted"))?;
        self.breakpoints.insert(address, id);
        Ok(format!("breakpoint #{id} set at 0x{address:016x}"))
    }

    fn add_watchpoint(&mut self, argument: &str, kind: Option<MemoryAccessKind>) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (address_text, width) = match parts.as_slice() {
            [address] => (*address, 1),
            [address, width] => (
                *address,
                width.parse::<u64>().map_err(|_| {
                    Diagnostic::error("MON-DBG-006", "watch width must be an unsigned integer")
                })?,
            ),
            _ => {
                return Err(Diagnostic::error(
                    "MON-DBG-005",
                    "watch expects <address> [width]",
                ));
            }
        };
        if !matches!(width, 1 | 2 | 4 | 8) {
            return Err(Diagnostic::error(
                "MON-DBG-006",
                "watch width must be 1, 2, 4, or 8 bytes",
            ));
        }
        let address = self.resolve_address(address_text, "MON-DBG-007")?;
        let id = self.next_watchpoint_id;
        self.next_watchpoint_id = id
            .checked_add(1)
            .ok_or_else(|| Diagnostic::error("MON-DBG-008", "watchpoint id exhausted"))?;
        self.watchpoints.insert(
            id,
            Watchpoint {
                address,
                width,
                kind,
                id,
            },
        );
        let mode = match kind {
            Some(MemoryAccessKind::Read) => "read",
            Some(MemoryAccessKind::Write) => "write",
            None => "access",
        };
        Ok(format!(
            "watchpoint #{id} set ({mode}) at 0x{address:016x} width={width}"
        ))
    }

    fn delete_debug_item(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (kind, id_text) = match parts.as_slice() {
            [id] => ("break", *id),
            [kind, id] if *kind == "break" || *kind == "watch" => (*kind, *id),
            _ => {
                return Err(Diagnostic::error(
                    "MON-DBG-009",
                    "delete expects [break|watch] <number>",
                ));
            }
        };
        let id = id_text.parse::<u64>().map_err(|_| {
            Diagnostic::error(
                "MON-DBG-010",
                "debug item number must be an unsigned integer",
            )
        })?;
        if kind == "watch" {
            if self.watchpoints.remove(&id).is_none() {
                return Err(Diagnostic::error(
                    "MON-DBG-011",
                    "watchpoint does not exist",
                ));
            }
            return Ok(format!("watchpoint #{id} deleted"));
        }
        let address = self
            .breakpoints
            .iter()
            .find_map(|(address, current_id)| (*current_id == id).then_some(*address))
            .ok_or_else(|| Diagnostic::error("MON-DBG-012", "breakpoint does not exist"))?;
        self.breakpoints.remove(&address);
        Ok(format!("breakpoint #{id} deleted"))
    }

    fn info_debug(&self, argument: &str) -> Result<String> {
        match argument.trim() {
            "break" | "breakpoints" | "b" => self.info_breakpoints(),
            "watch" | "watchpoints" | "w" => self.info_watchpoints(),
            "" => Err(Diagnostic::error(
                "MON-DBG-013",
                "info expects break or watch",
            )),
            _ => Err(Diagnostic::error(
                "MON-DBG-013",
                "info expects break or watch",
            )),
        }
    }

    fn info_breakpoints(&self) -> Result<String> {
        if self.breakpoints.is_empty() {
            return Ok("breakpoints: none".into());
        }
        let mut output = String::from("breakpoints:\n");
        for (address, id) in &self.breakpoints {
            writeln!(output, "  #{id} addr=0x{address:016x}").unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn info_watchpoints(&self) -> Result<String> {
        if self.watchpoints.is_empty() {
            return Ok("watchpoints: none".into());
        }
        let mut output = String::from("watchpoints:\n");
        for watchpoint in self.watchpoints.values() {
            let mode = match watchpoint.kind {
                Some(MemoryAccessKind::Read) => "read",
                Some(MemoryAccessKind::Write) => "write",
                None => "access",
            };
            writeln!(
                output,
                "  #{} {mode} addr=0x{:016x} width={}",
                watchpoint.id, watchpoint.address, watchpoint.width
            )
            .unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn watchpoint_hit(&self, access: MemoryAccess) -> Option<&Watchpoint> {
        let access_end = access.address.checked_add(u64::from(access.width))?;
        self.watchpoints.values().find(|watchpoint| {
            let watch_end = watchpoint.address.checked_add(watchpoint.width);
            let kind_matches = watchpoint.kind.is_none() || watchpoint.kind == Some(access.kind);
            kind_matches
                && watch_end.is_some_and(|watch_end| {
                    access.address < watch_end && watchpoint.address < access_end
                })
        })
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

/// Backend-neutral command surface used by targets that are not owned by the
/// host simulator, such as the QEMU GDB remote backend.
///
/// The legacy [`Monitor`] remains the richer host profile for now. This type
/// deliberately contains only operations whose state and semantics are
/// available through [`TargetBackend`]; it is the migration seam for the
/// eventual full debugger UI.
pub struct BackendConsole<B> {
    pub backend: B,
    symbols: BTreeMap<u64, String>,
    view_address: u64,
    undo: Vec<MemoryEdit>,
    breakpoints: BTreeMap<u64, BackendBreakpoint>,
    next_breakpoint_id: u64,
    watchpoints: BTreeMap<u64, Watchpoint>,
    next_watchpoint_id: u64,
    history: Vec<HistoryEntry>,
    next_history_sequence: u64,
    max_run_steps: u64,
}

impl<B> BackendConsole<B>
where
    B: TargetBackend,
    B::Error: std::fmt::Debug,
{
    pub fn new(backend: B) -> Self {
        let view_address = backend.context().pc;
        Self {
            backend,
            symbols: BTreeMap::new(),
            view_address,
            undo: Vec::new(),
            breakpoints: BTreeMap::new(),
            next_breakpoint_id: 1,
            watchpoints: BTreeMap::new(),
            next_watchpoint_id: 1,
            history: Vec::new(),
            next_history_sequence: 1,
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
            "help" | "?" => Ok(backend_help()),
            "assemble" | "a" => self.assemble(argument),
            "assemble-program" | "load" => self.assemble_program(argument),
            "step" | "s" => self.step(),
            "run" | "r" => self.run(argument),
            "continue" | "c" => self.continue_target(argument),
            "regs" | "registers" => Ok(self.registers()),
            "disasm" | "d" => self.disassemble(argument),
            "symbols" => self.show_symbols(),
            "where" => Ok(self.show_location()),
            "memory" | "mem" | "hex" | "ascii" => self.memory(argument),
            "view" | "jump" => self.set_view(argument),
            "edit" | "e" => self.edit(argument),
            "undo" | "u" => self.undo_edit(),
            "break" | "b" => self.add_breakpoint(argument),
            "delete" | "del" => self.delete_breakpoint(argument),
            "watch" => self.add_watchpoint(argument, Some(MemoryAccessKind::Write)),
            "rwatch" => self.add_watchpoint(argument, Some(MemoryAccessKind::Read)),
            "awatch" => self.add_watchpoint(argument, None),
            "info" => self.info(argument),
            "history" | "trace" => self.show_history(argument),
            "quit" | "exit" => Ok("bye".into()),
            _ => Err(Diagnostic::error(
                "MON-CMD-101",
                format!("unknown backend command: {name}; use help"),
            )),
        }
    }

    fn assemble(&mut self, source: &str) -> Result<String> {
        if source.is_empty() {
            return Err(Diagnostic::error(
                "MON-ASM-101",
                "assemble expects one source line",
            ));
        }
        let image = assemble(source)?;
        let address = self.backend.context().pc;
        self.backend
            .write_memory(address, &image.text)
            .map_err(target_error)?;
        self.view_address = address;
        Ok(format!(
            "loaded {} byte(s) at 0x{address:016x}",
            image.text.len()
        ))
    }

    fn assemble_program(&mut self, source: &str) -> Result<String> {
        if source.trim().is_empty() {
            return Err(Diagnostic::error(
                "MON-ASM-102",
                "assemble-program expects at least one source line",
            ));
        }
        let image = assemble_source_program(source)?;
        let base = self.backend.context().pc;
        self.backend
            .write_memory(base, &image.text)
            .map_err(target_error)?;
        let mut symbols = BTreeMap::new();
        for (name, offset) in image.symbols {
            let address = base
                .checked_add(offset)
                .ok_or_else(|| Diagnostic::error("MON-ASM-103", "symbol address overflow"))?;
            symbols.insert(address, name);
        }
        self.symbols = symbols;
        self.view_address = base;
        let entry = base
            .checked_add(image.entry)
            .ok_or_else(|| Diagnostic::error("MON-ASM-103", "entry address overflow"))?;
        Ok(format!(
            "loaded {} byte(s) at 0x{base:016x}; entry=0x{entry:016x}; {} symbol(s)",
            image.text.len(),
            self.symbols.len()
        ))
    }

    fn step(&mut self) -> Result<String> {
        let pc_before = self.backend.context().pc;
        let word = self.read_word(pc_before)?;
        let text = disassemble_word(pc_before, word, &self.symbols).text;
        let outcome = self.backend.step().map_err(target_error)?;
        if let ExecutionOutcome::Retired {
            pc_after,
            memory_access,
            ..
        } = outcome
        {
            self.record_retired(pc_before, pc_after, word, memory_access);
            if let Some(access) = memory_access {
                if let Some(watchpoint) = self.watchpoint_hit(access) {
                    return Ok(format_watchpoint_stop(
                        watchpoint,
                        access,
                        self.backend.context().pc,
                        self.next_history_sequence.saturating_sub(1),
                    ));
                }
            }
        }
        Ok(format_backend_console_step(pc_before, word, &text, outcome))
    }

    fn run(&mut self, argument: &str) -> Result<String> {
        let limit = parse_run_limit(argument, self.max_run_steps)?;
        self.run_with_limit(limit, false)
    }

    fn continue_target(&mut self, argument: &str) -> Result<String> {
        let limit = parse_run_limit(argument, self.max_run_steps)?;
        self.run_with_limit(limit, true)
    }

    fn run_with_limit(&mut self, limit: u64, bypass_current: bool) -> Result<String> {
        let mut bypass =
            bypass_current && self.breakpoints.contains_key(&self.backend.context().pc);
        let mut executed = 0u64;
        for _ in 0..limit {
            let pc = self.backend.context().pc;
            if !bypass {
                if let Some(breakpoint) = self.breakpoints.get(&pc) {
                    return Ok(format!(
                        "stopped: breakpoint #{} at pc=0x{pc:016x}",
                        breakpoint.id
                    ));
                }
            }
            let word = self.read_word(pc)?;
            let outcome = self.backend.step().map_err(target_error)?;
            executed = executed.saturating_add(1);
            match outcome {
                ExecutionOutcome::Retired {
                    pc_after,
                    memory_access,
                    ..
                } => {
                    self.record_retired(pc, pc_after, word, memory_access);
                    bypass = false;
                    if let Some(access) = memory_access {
                        if let Some(watchpoint) = self.watchpoint_hit(access) {
                            return Ok(format_watchpoint_stop(
                                watchpoint,
                                access,
                                self.backend.context().pc,
                                self.next_history_sequence.saturating_sub(1),
                            ));
                        }
                    }
                }
                ExecutionOutcome::Stopped(event) => {
                    return Ok(format_backend_stop(event));
                }
                ExecutionOutcome::BudgetExhausted { .. } => {
                    return Err(Diagnostic::error(
                        "MON-DEBUG-101",
                        "step backend returned a run-only outcome",
                    ));
                }
            }
        }
        Ok(format!(
            "ran {executed} step(s); pc=0x{:016x}",
            self.backend.context().pc
        ))
    }

    fn registers(&self) -> String {
        let context = self.backend.context();
        let mut output = String::from("integer registers\n");
        for row in 0..8 {
            for column in 0..4 {
                let register = row * 4 + column;
                write!(output, "x{register:02}=0x{:016x}  ", context.x[register]).unwrap();
            }
            output.push('\n');
        }
        output.push_str("floating registers (raw bits)\n");
        for row in 0..8 {
            for column in 0..4 {
                let register = row * 4 + column;
                write!(output, "f{register:02}=0x{:016x}  ", context.f[register]).unwrap();
            }
            output.push('\n');
        }
        writeln!(
            output,
            "pc=0x{:016x} fcsr=0x{:08x} capabilities={:?}",
            context.pc,
            context.fcsr,
            self.backend.capabilities()
        )
        .unwrap();
        output.trim_end().into()
    }

    fn disassemble(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (address, count) = match parts.as_slice() {
            [] => (self.backend.context().pc, 4),
            [count] => (
                self.backend.context().pc,
                parse_count(count, "MON-DISASM-101")?,
            ),
            [address, count] => (
                self.resolve_address(address, "MON-DISASM-102")?,
                parse_count(count, "MON-DISASM-101")?,
            ),
            _ => {
                return Err(Diagnostic::error(
                    "MON-DISASM-101",
                    "disasm expects [addr] [count]",
                ));
            }
        };
        if count > MAX_MEMORY_VIEW_BYTES / 4 {
            return Err(Diagnostic::error(
                "MON-DISASM-103",
                "disassembly count exceeds the 1024-instruction interactive limit",
            ));
        }
        let mut output = String::new();
        for index in 0..count {
            let address = address
                .checked_add(
                    (index as u64)
                        .checked_mul(4)
                        .ok_or_else(|| Diagnostic::error("MON-DISASM-102", "address overflow"))?,
                )
                .ok_or_else(|| Diagnostic::error("MON-DISASM-102", "address overflow"))?;
            let word = self.read_word(address)?;
            let line = disassemble_word(address, word, &self.symbols);
            writeln!(output, "0x{address:016x}: {word:08x}  {}", line.text).unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn show_symbols(&self) -> Result<String> {
        if self.symbols.is_empty() {
            return Ok("symbols: none".into());
        }
        let mut output = String::from("symbols:\n");
        for (address, name) in &self.symbols {
            writeln!(output, "  0x{address:016x} {name}").unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn show_location(&self) -> String {
        let pc = self.backend.context().pc;
        let symbol = self
            .symbols
            .range(..=pc)
            .next_back()
            .map(|(address, name)| {
                if *address == pc {
                    name.clone()
                } else {
                    format!("{name}+0x{:x}", pc - address)
                }
            })
            .unwrap_or_else(|| "<no-symbol>".into());
        format!("pc=0x{pc:016x} {symbol} view=0x{:016x}", self.view_address)
    }

    fn resolve_address(&self, value: &str, code: &'static str) -> Result<u64> {
        let name = value.strip_prefix('@').unwrap_or(value);
        if let Some(address) = self
            .symbols
            .iter()
            .find_map(|(address, symbol)| (symbol == name).then_some(*address))
        {
            return Ok(address);
        }
        parse_address(value, code)
    }

    fn memory(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (address, count) = match parts.as_slice() {
            [] => (self.view_address, DEFAULT_MEMORY_VIEW_BYTES),
            [address] => (
                parse_address(address, "MON-MEM-101")?,
                DEFAULT_MEMORY_VIEW_BYTES,
            ),
            [address, count] => (
                parse_address(address, "MON-MEM-101")?,
                parse_count(count, "MON-MEM-102")?,
            ),
            _ => {
                return Err(Diagnostic::error(
                    "MON-MEM-100",
                    "memory expects [address] [count]",
                ));
            }
        };
        if count > MAX_MEMORY_VIEW_BYTES {
            return Err(Diagnostic::error(
                "MON-MEM-103",
                "memory view exceeds the 4096-byte interactive limit",
            ));
        }
        let mut bytes = vec![0u8; count];
        self.backend
            .read_memory(address, &mut bytes)
            .map_err(target_error)?;
        self.view_address = address;
        format_memory_view(address, &bytes)
    }

    fn set_view(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let [address] = parts.as_slice() else {
            return Err(Diagnostic::error(
                "MON-MEM-104",
                "view expects one target address",
            ));
        };
        self.view_address = parse_address(address, "MON-MEM-105")?;
        Ok(format!("view=0x{:016x}", self.view_address))
    }

    fn edit(&mut self, argument: &str) -> Result<String> {
        let mut parts = argument.split_whitespace();
        let address = parts
            .next()
            .ok_or_else(|| Diagnostic::error("MON-MEM-106", "edit expects an address and bytes"))
            .and_then(|value| parse_address(value, "MON-MEM-101"))?;
        let mut bytes = Vec::new();
        for token in parts {
            parse_byte_token(token, &mut bytes)?;
            if bytes.len() > MAX_EDIT_BYTES {
                return Err(Diagnostic::error(
                    "MON-MEM-107",
                    "edit exceeds the 4096-byte transaction limit",
                ));
            }
        }
        if bytes.is_empty() {
            return Err(Diagnostic::error(
                "MON-MEM-106",
                "edit expects at least one byte",
            ));
        }
        let mut previous = vec![0u8; bytes.len()];
        self.backend
            .read_memory(address, &mut previous)
            .map_err(target_error)?;
        self.backend
            .write_memory(address, &bytes)
            .map_err(target_error)?;
        self.undo.push(MemoryEdit { address, previous });
        self.view_address = address;
        Ok(format!(
            "edited {} byte(s) at 0x{address:016x}",
            bytes.len()
        ))
    }

    fn undo_edit(&mut self) -> Result<String> {
        let edit = self
            .undo
            .pop()
            .ok_or_else(|| Diagnostic::error("MON-MEM-108", "nothing to undo"))?;
        self.backend
            .write_memory(edit.address, &edit.previous)
            .map_err(target_error)?;
        self.view_address = edit.address;
        Ok(format!("undid edit at 0x{:016x}", edit.address))
    }

    fn add_breakpoint(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let [address_text] = parts.as_slice() else {
            return Err(Diagnostic::error(
                "MON-DEBUG-100",
                "break expects one target address",
            ));
        };
        let address = parse_address(address_text, "MON-DEBUG-101")?;
        if address & 3 != 0 {
            return Err(Diagnostic::error(
                "MON-DEBUG-102",
                "backend breakpoints require four-byte alignment",
            ));
        }
        if self.breakpoints.contains_key(&address) {
            return Err(Diagnostic::error(
                "MON-DEBUG-103",
                "a breakpoint already exists at this address",
            ));
        }
        let id = self.next_breakpoint_id;
        self.next_breakpoint_id = self.next_breakpoint_id.saturating_add(1);
        self.breakpoints
            .insert(address, BackendBreakpoint { id, address });
        Ok(format!("breakpoint #{id} set at 0x{address:016x}"))
    }

    fn delete_breakpoint(&mut self, argument: &str) -> Result<String> {
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (kind, id_text) = match parts.as_slice() {
            [id] => ("break", *id),
            [kind, id] if *kind == "break" || *kind == "breakpoint" => ("break", *id),
            [kind, id] if *kind == "watch" || *kind == "watchpoint" => ("watch", *id),
            _ => {
                return Err(Diagnostic::error(
                    "MON-DEBUG-104",
                    "delete expects <id>, break <id>, or watch <id>",
                ));
            }
        };
        let id = parse_count(id_text, "MON-DEBUG-105")? as u64;
        if kind == "watch" {
            self.watchpoints
                .remove(&id)
                .ok_or_else(|| Diagnostic::error("MON-DEBUG-113", "watchpoint does not exist"))?;
            return Ok(format!("watchpoint #{id} deleted"));
        }
        let address = self
            .breakpoints
            .iter()
            .find_map(|(address, breakpoint)| (breakpoint.id == id).then_some(*address))
            .ok_or_else(|| Diagnostic::error("MON-DEBUG-106", "breakpoint does not exist"))?;
        self.breakpoints.remove(&address);
        Ok(format!("breakpoint #{id} deleted"))
    }

    fn info(&self, argument: &str) -> Result<String> {
        if argument == "watch" || argument == "watchpoints" {
            return self.info_watchpoints();
        }
        if argument == "history" || argument == "trace" {
            return Ok(format!("history: {} entr(ies)", self.history.len()));
        }
        if !argument.is_empty() && argument != "break" && argument != "breakpoints" {
            return Err(Diagnostic::error(
                "MON-DEBUG-107",
                "info accepts break or breakpoints",
            ));
        }
        if self.breakpoints.is_empty() {
            return Ok("breakpoints: none".into());
        }
        let mut output = String::from("breakpoints:\n");
        for breakpoint in self.breakpoints.values() {
            writeln!(
                output,
                "  #{} at 0x{:016x}",
                breakpoint.id, breakpoint.address
            )
            .unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn add_watchpoint(&mut self, argument: &str, kind: Option<MemoryAccessKind>) -> Result<String> {
        if !self.backend.capabilities().supports_watchpoints {
            return Err(Diagnostic::error(
                "MON-DEBUG-109",
                "backend does not expose memory access events for watchpoints",
            ));
        }
        let parts: Vec<_> = argument.split_whitespace().collect();
        let (address_text, width) = match parts.as_slice() {
            [address] => (*address, 1),
            [address, width] => (
                *address,
                width.parse::<u64>().map_err(|_| {
                    Diagnostic::error("MON-DEBUG-110", "watch width must be an unsigned integer")
                })?,
            ),
            _ => {
                return Err(Diagnostic::error(
                    "MON-DEBUG-110",
                    "watch expects <address> [width]",
                ));
            }
        };
        if !matches!(width, 1 | 2 | 4 | 8) {
            return Err(Diagnostic::error(
                "MON-DEBUG-110",
                "watch width must be 1, 2, 4, or 8 bytes",
            ));
        }
        let address = parse_address(address_text, "MON-DEBUG-111")?;
        let id = self.next_watchpoint_id;
        self.next_watchpoint_id = id
            .checked_add(1)
            .ok_or_else(|| Diagnostic::error("MON-DEBUG-112", "watchpoint id exhausted"))?;
        self.watchpoints.insert(
            id,
            Watchpoint {
                address,
                width,
                kind,
                id,
            },
        );
        let mode = match kind {
            Some(MemoryAccessKind::Read) => "read",
            Some(MemoryAccessKind::Write) => "write",
            None => "access",
        };
        Ok(format!(
            "watchpoint #{id} set ({mode}) at 0x{address:016x} width={width}"
        ))
    }

    fn info_watchpoints(&self) -> Result<String> {
        if self.watchpoints.is_empty() {
            return Ok("watchpoints: none".into());
        }
        let mut output = String::from("watchpoints:\n");
        for watchpoint in self.watchpoints.values() {
            let mode = match watchpoint.kind {
                Some(MemoryAccessKind::Read) => "read",
                Some(MemoryAccessKind::Write) => "write",
                None => "access",
            };
            writeln!(
                output,
                "  #{} {mode} at 0x{:016x} width={}",
                watchpoint.id, watchpoint.address, watchpoint.width
            )
            .unwrap();
        }
        Ok(output.trim_end().into())
    }

    fn watchpoint_hit(&self, access: MemoryAccess) -> Option<&Watchpoint> {
        let access_end = access.address.checked_add(u64::from(access.width))?;
        self.watchpoints.values().find(|watchpoint| {
            let watch_end = watchpoint.address.checked_add(watchpoint.width);
            let kind_matches = watchpoint.kind.is_none() || watchpoint.kind == Some(access.kind);
            kind_matches
                && watch_end.is_some_and(|watch_end| {
                    access.address < watch_end && watchpoint.address < access_end
                })
        })
    }

    fn record_retired(
        &mut self,
        pc_before: u64,
        pc_after: u64,
        word: u32,
        memory_access: Option<MemoryAccess>,
    ) {
        if self.history.len() == MAX_HISTORY_ENTRIES {
            self.history.remove(0);
        }
        let sequence = self.next_history_sequence;
        self.next_history_sequence = sequence.saturating_add(1);
        self.history.push(HistoryEntry {
            sequence,
            pc_before,
            pc_after,
            instruction: disassemble_word(pc_before, word, &self.symbols).text,
            memory_access,
        });
    }

    fn show_history(&self, argument: &str) -> Result<String> {
        let requested = if argument.is_empty() {
            16
        } else {
            parse_count(argument, "MON-HIST-101")?
        };
        let count = requested.min(256).min(self.history.len());
        if count == 0 {
            return Ok("history: empty".into());
        }
        let start = self.history.len() - count;
        let mut output = String::from("history:\n");
        for entry in &self.history[start..] {
            write!(
                output,
                "  #{:06}  0x{:016x} -> 0x{:016x}  {}",
                entry.sequence, entry.pc_before, entry.pc_after, entry.instruction
            )
            .unwrap();
            if let Some(access) = entry.memory_access {
                let kind = match access.kind {
                    MemoryAccessKind::Read => "read",
                    MemoryAccessKind::Write => "write",
                };
                write!(
                    output,
                    " [{kind} 0x{:016x}/{}]",
                    access.address, access.width
                )
                .unwrap();
            }
            output.push('\n');
        }
        Ok(output.trim_end().into())
    }

    fn read_word(&mut self, address: u64) -> Result<u32> {
        let mut bytes = [0u8; 4];
        self.backend
            .read_memory(address, &mut bytes)
            .map_err(target_error)?;
        Ok(u32::from_le_bytes(bytes))
    }
}

fn target_error<E: std::fmt::Debug>(error: E) -> Diagnostic {
    Diagnostic::error(
        "MON-TARGET-001",
        format!("target operation failed: {error:?}"),
    )
}

fn format_backend_console_step(
    pc: u64,
    word: u32,
    text: &str,
    outcome: ExecutionOutcome,
) -> String {
    format!(
        "0x{pc:016x}: {word:08x}  {text} -> {}",
        format_backend_console_outcome(outcome)
    )
}

fn format_backend_console_outcome(outcome: ExecutionOutcome) -> String {
    match outcome {
        ExecutionOutcome::Retired { pc_after, .. } => format!("pc=0x{pc_after:016x}"),
        ExecutionOutcome::Stopped(event) => format_backend_stop(event),
        ExecutionOutcome::BudgetExhausted {
            pc,
            instruction_count,
        } => format!("budget exhausted at pc=0x{pc:016x}; total={instruction_count}"),
    }
}

fn format_memory_view(address: u64, bytes: &[u8]) -> Result<String> {
    let mut output = String::new();
    for (row, chunk) in bytes.chunks(16).enumerate() {
        let row_address = address
            .checked_add((row * 16) as u64)
            .ok_or_else(|| Diagnostic::error("MON-MEM-101", "address overflow"))?;
        write!(output, "0x{row_address:016x}: ").unwrap();
        for byte in chunk {
            write!(output, "{byte:02x} ").unwrap();
        }
        for _ in chunk.len()..16 {
            output.push_str("   ");
        }
        output.push('|');
        for byte in chunk {
            output.push(if (0x20..=0x7e).contains(byte) {
                *byte as char
            } else {
                '.'
            });
        }
        output.push('|');
        output.push('\n');
    }
    Ok(output.trim_end().into())
}

fn backend_help() -> String {
    [
        "help                 show backend-neutral commands",
        "assemble <source>    assemble and write at target pc",
        "assemble-program <source> load a multi-line image and symbols",
        "step                 execute one target instruction",
        "run [count]          execute up to count target steps",
        "continue [count]     resume, bypassing a breakpoint at current pc",
        "regs                 show target registers and capabilities",
        "disasm [addr] [n]    disassemble target words with symbols",
        "symbols              list loaded symbols",
        "where                show pc, nearest symbol and memory view",
        "memory [addr] [n]    show target memory as hex/ASCII",
        "view <addr>          move memory view without changing pc",
        "edit <addr> <bytes>  write target bytes transactionally",
        "undo                 restore the last target memory edit",
        "break <addr>         stop before a target address",
        "delete <id>          remove a backend breakpoint",
        "info break           list backend breakpoints",
        "watch <addr> [w]     stop on target memory writes",
        "rwatch <addr> [w]    stop on target memory reads",
        "awatch <addr> [w]    stop on target reads or writes",
        "info watch           list backend watchpoints",
        "history [count]      show bounded target execution history",
        "quit                 close the console",
    ]
    .join("\n")
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

fn parse_run_limit(argument: &str, default: u64) -> Result<u64> {
    if argument.is_empty() {
        return Ok(default);
    }
    argument
        .parse::<u64>()
        .map_err(|_| Diagnostic::error("MON-RUN-001", "run count must be an unsigned integer"))
}

fn format_backend_stop(event: StopEvent) -> String {
    format!(
        "stopped: {:?} at pc=0x{:016x}; total={}",
        event.reason, event.pc, event.instruction_count
    )
}

fn format_watchpoint_stop(
    watchpoint: &Watchpoint,
    access: MemoryAccess,
    pc: u64,
    instruction_count: u64,
) -> String {
    let mode = match access.kind {
        MemoryAccessKind::Read => "read",
        MemoryAccessKind::Write => "write",
    };
    format!(
        "stopped: watchpoint #{} at pc=0x{pc:016x}; {mode} addr=0x{:016x} width={}; total={instruction_count}",
        watchpoint.id, access.address, access.width
    )
}

fn help() -> String {
    [
        "help                 show commands",
        "assemble <source>    assemble and load one source line at pc",
        "assemble-program <source> load a multi-line program and symbols",
        "step                 execute one instruction",
        "run [count]          execute up to count instructions (default 1000)",
        "continue [count]     resume, bypassing a breakpoint at current pc",
        "disasm [addr] [count] show instructions (default pc, 4)",
        "memory [addr] [count] show hex and ASCII (default view, 64)",
        "view <addr>          move memory view without changing pc",
        "edit <addr> <bytes>  write bytes transactionally (hex)",
        "undo                 undo the last memory edit",
        "mark <name> [addr]   name the current or specified address",
        "marks                list named addresses",
        "unmark <name>        remove a named address",
        "break <addr>         add a logical breakpoint",
        "delete [kind] <id>   delete breakpoint or watchpoint",
        "info break|watch     list debugger stops",
        "watch <addr> [w]     stop on memory writes",
        "rwatch <addr> [w]    stop on memory reads",
        "awatch <addr> [w]    stop on reads or writes",
        "symbols              list loaded symbols",
        "where                show pc, nearest symbol and memory view",
        "stack                show inferred jal/jalr call stack",
        "history [count]      show bounded execution history",
        "snapshot <path>      save machine/debugger state",
        "restore <path>       restore a snapshot atomically",
        "project-save <path>  save source plus state",
        "project-load <path>  restore a project",
        "regs                 show x/f registers and fcsr exactly",
        "reset                reset machine state",
        "quit                 leave the interactive monitor",
    ]
    .join("\n")
}

fn parse_address(value: &str, code: &'static str) -> Result<u64> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.is_empty() {
        return Err(Diagnostic::error(code, "address is empty"));
    }
    if value.chars().all(|character| character.is_ascii_hexdigit()) {
        u64::from_str_radix(value, 16)
            .map_err(|_| Diagnostic::error(code, "address does not fit in 64 bits"))
    } else {
        Err(Diagnostic::error(code, "address must be hexadecimal"))
    }
}

fn parse_count(value: &str, code: &'static str) -> Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| Diagnostic::error(code, "count must be an unsigned decimal integer"))
}

fn parse_byte_token(token: &str, output: &mut Vec<u8>) -> Result<()> {
    let token = token.strip_prefix("0x").unwrap_or(token);
    if token.is_empty() || token.len() % 2 != 0 || !token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Diagnostic::error(
            "MON-MEM-008",
            "bytes must be one or more even hexadecimal digits",
        ));
    }
    for pair in token.as_bytes().chunks_exact(2) {
        let text = std::str::from_utf8(pair).expect("ASCII hexadecimal token");
        let byte = u8::from_str_radix(text, 16).expect("validated hexadecimal byte");
        output.push(byte);
    }
    Ok(())
}

fn validate_mark_name(name: &str) -> Result<()> {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return Err(Diagnostic::error("MON-MARK-006", "mark name is empty"));
    };
    if name.len() > 32
        || !(first == '_' || first.is_ascii_alphabetic())
        || !characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(Diagnostic::error(
            "MON-MARK-006",
            "mark name must match [A-Za-z_][A-Za-z0-9_]* and fit in 32 bytes",
        ));
    }
    Ok(())
}

struct ByteWriter {
    bytes: Vec<u8>,
}

impl ByteWriter {
    fn new(magic: &[u8; 8]) -> Self {
        Self {
            bytes: magic.to_vec(),
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn string(&mut self, value: &str) -> Result<()> {
        let length = u32::try_from(value.len())
            .map_err(|_| Diagnostic::error("MON-PERSIST-007", "string is too large"))?;
        self.u32(length);
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn map_u64_string(&mut self, values: &BTreeMap<u64, String>) -> Result<()> {
        self.u32(
            u32::try_from(values.len())
                .map_err(|_| Diagnostic::error("MON-PERSIST-008", "symbol table is too large"))?,
        );
        for (address, value) in values {
            self.u64(*address);
            self.string(value)?;
        }
        Ok(())
    }

    fn map_string_u64(&mut self, values: &BTreeMap<String, u64>) -> Result<()> {
        self.u32(
            u32::try_from(values.len())
                .map_err(|_| Diagnostic::error("MON-PERSIST-009", "mark table is too large"))?,
        );
        for (name, address) in values {
            self.string(name)?;
            self.u64(*address);
        }
        Ok(())
    }

    fn bytes(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8], magic: &[u8; 8]) -> Result<Self> {
        if bytes.len() < magic.len() || &bytes[..magic.len()] != magic {
            return Err(Diagnostic::error(
                "MON-PERSIST-010",
                "unrecognized persistence magic",
            ));
        }
        Ok(Self {
            bytes,
            position: magic.len(),
        })
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| Diagnostic::error("MON-PERSIST-011", "persistence offset overflow"))?;
        if end > self.bytes.len() {
            return Err(Diagnostic::error(
                "MON-PERSIST-012",
                "truncated persistence file",
            ));
        }
        let value = &self.bytes[self.position..end];
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }

    fn string(&mut self) -> Result<String> {
        let length = self.u32()? as usize;
        if length > MAX_PERSISTED_BYTES {
            return Err(Diagnostic::error(
                "MON-PERSIST-013",
                "persisted string exceeds size limit",
            ));
        }
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| {
            Diagnostic::error("MON-PERSIST-014", "persisted string is not valid UTF-8")
        })
    }

    fn bytes_vec(&mut self, length: usize) -> Result<Vec<u8>> {
        if length > MAX_PERSISTED_BYTES {
            return Err(Diagnostic::error(
                "MON-PERSIST-015",
                "persisted byte array exceeds size limit",
            ));
        }
        Ok(self.take(length)?.to_vec())
    }

    fn finish(self) -> Result<()> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(Diagnostic::error(
                "MON-PERSIST-016",
                "trailing bytes in persistence file",
            ))
        }
    }
}

fn decode_snapshot(bytes: &[u8]) -> Result<DecodedSnapshot> {
    if bytes.len() > MAX_PERSISTED_BYTES {
        return Err(Diagnostic::error(
            "MON-PERSIST-017",
            "snapshot exceeds size limit",
        ));
    }
    let mut reader = ByteReader::new(bytes, SNAPSHOT_MAGIC)?;
    if reader.u32()? != PERSISTENCE_VERSION {
        return Err(Diagnostic::error(
            "MON-PERSIST-018",
            "unsupported snapshot version",
        ));
    }
    let memory_length = usize::try_from(reader.u64()?)
        .map_err(|_| Diagnostic::error("MON-PERSIST-019", "invalid memory length"))?;
    let mut x = [0u64; 32];
    let mut f = [0u64; 32];
    for value in &mut x {
        *value = reader.u64()?;
    }
    for value in &mut f {
        *value = reader.u64()?;
    }
    let fcsr = reader.u32()?;
    let pc = reader.u64()?;
    let instructions = reader.u64()?;
    let view_address = reader.u64()?;
    let next_breakpoint_id = reader.u64()?;
    let next_watchpoint_id = reader.u64()?;

    let symbol_count = checked_count(reader.u32()?, "symbol table")?;
    let mut symbols = BTreeMap::new();
    for _ in 0..symbol_count {
        let address = reader.u64()?;
        let name = reader.string()?;
        if symbols.insert(address, name).is_some() {
            return Err(Diagnostic::error(
                "MON-PERSIST-020",
                "duplicate symbol address",
            ));
        }
    }
    let mark_count = checked_count(reader.u32()?, "mark table")?;
    let mut marks = BTreeMap::new();
    for _ in 0..mark_count {
        let name = reader.string()?;
        let address = reader.u64()?;
        if marks.insert(name, address).is_some() {
            return Err(Diagnostic::error("MON-PERSIST-021", "duplicate mark name"));
        }
    }
    let breakpoint_count = checked_count(reader.u32()?, "breakpoint table")?;
    let mut breakpoints = BTreeMap::new();
    for _ in 0..breakpoint_count {
        let address = reader.u64()?;
        let id = reader.u64()?;
        if breakpoints.insert(address, id).is_some() {
            return Err(Diagnostic::error(
                "MON-PERSIST-022",
                "duplicate breakpoint address",
            ));
        }
    }
    let watchpoint_count = checked_count(reader.u32()?, "watchpoint table")?;
    let mut watchpoints = BTreeMap::new();
    for _ in 0..watchpoint_count {
        let id = reader.u64()?;
        let address = reader.u64()?;
        let width = reader.u64()?;
        let kind = match reader.u8()? {
            0 => None,
            1 => Some(MemoryAccessKind::Read),
            2 => Some(MemoryAccessKind::Write),
            _ => {
                return Err(Diagnostic::error(
                    "MON-PERSIST-023",
                    "invalid watchpoint access kind",
                ));
            }
        };
        if watchpoints
            .insert(
                id,
                Watchpoint {
                    address,
                    width,
                    kind,
                    id,
                },
            )
            .is_some()
        {
            return Err(Diagnostic::error(
                "MON-PERSIST-024",
                "duplicate watchpoint id",
            ));
        }
    }
    let memory = reader.bytes_vec(memory_length)?;
    reader.finish()?;

    let mut machine = Machine::new(memory_length);
    machine.load(0, &memory)?;
    machine.x = x;
    machine.f = f;
    machine.fcsr = fcsr;
    machine.pc = pc;
    machine.instructions = instructions;
    Ok(DecodedSnapshot {
        machine,
        symbols,
        marks,
        breakpoints,
        watchpoints,
        next_breakpoint_id,
        next_watchpoint_id,
        view_address,
    })
}

fn decode_project(bytes: &[u8]) -> Result<(String, Vec<u8>)> {
    if bytes.len() > MAX_PERSISTED_BYTES {
        return Err(Diagnostic::error(
            "MON-PERSIST-025",
            "project exceeds size limit",
        ));
    }
    let mut reader = ByteReader::new(bytes, PROJECT_MAGIC)?;
    if reader.u32()? != PERSISTENCE_VERSION {
        return Err(Diagnostic::error(
            "MON-PERSIST-026",
            "unsupported project version",
        ));
    }
    let source = reader.string()?;
    let snapshot_length = usize::try_from(reader.u64()?)
        .map_err(|_| Diagnostic::error("MON-PERSIST-027", "invalid snapshot length"))?;
    let snapshot = reader.bytes_vec(snapshot_length)?;
    reader.finish()?;
    decode_snapshot(&snapshot)?;
    Ok((source, snapshot))
}

fn checked_count(value: u32, name: &str) -> Result<usize> {
    let count = usize::try_from(value)
        .map_err(|_| Diagnostic::error("MON-PERSIST-028", format!("invalid {name} count")))?;
    if count > 1_000_000 {
        return Err(Diagnostic::error(
            "MON-PERSIST-029",
            format!("{name} count exceeds limit"),
        ));
    }
    Ok(count)
}

fn persistence_path<'a>(argument: &'a str, code: &'static str) -> Result<&'a Path> {
    let path = argument.trim();
    if path.is_empty() || path.contains('\0') {
        return Err(Diagnostic::error(
            code,
            "persistence command expects a valid path",
        ));
    }
    Ok(Path::new(path))
}

fn read_persistence_file(path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(path).map_err(|error| {
        Diagnostic::error(
            "MON-PERSIST-030",
            format!("cannot read persistence file: {error}"),
        )
    })?;
    if bytes.len() > MAX_PERSISTED_BYTES {
        return Err(Diagnostic::error(
            "MON-PERSIST-031",
            "persistence file exceeds size limit",
        ));
    }
    Ok(bytes)
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

    #[test]
    fn memory_view_keeps_hex_ascii_and_pc_navigation_separate() {
        let mut monitor = Monitor::new(128);
        monitor.execute("assemble addi x1,x0,1").unwrap();
        let output = monitor.execute("memory 0x0 4").unwrap();
        assert!(output.contains("0x0000000000000000: 93 00 10 00"));
        assert!(output.contains("|....|"));

        monitor.execute("view 0x20").unwrap();
        assert_eq!(monitor.machine.pc, 0);
        assert!(monitor.execute("memory 4").is_ok());
        assert_eq!(monitor.view_address, 4);
    }

    #[test]
    fn memory_edit_and_undo_restore_bytes_through_backend() {
        let mut monitor = Monitor::new(128);
        monitor.execute("edit 0x10 deadbeef").unwrap();
        assert_eq!(monitor.machine.memory.load32(0x10).unwrap(), 0xefbe_adde);
        let edited = monitor.execute("memory 0x10 4").unwrap();
        assert!(edited.contains("de ad be ef"));

        let undo = monitor.execute("undo").unwrap();
        assert!(undo.contains("undid 4 byte(s)"));
        assert_eq!(monitor.machine.memory.load32(0x10).unwrap(), 0);
        assert!(monitor.execute("undo").is_err());
    }

    #[test]
    fn invalid_memory_edit_has_no_side_effect() {
        let mut monitor = Monitor::new(128);
        let error = monitor.execute("edit 0x10 123").unwrap_err();
        assert_eq!(error.code, "MON-MEM-008");
        assert_eq!(monitor.machine.memory.load32(0x10).unwrap(), 0);
    }

    #[test]
    fn marks_and_quickjump_resolve_without_changing_pc() {
        let mut monitor = Monitor::new(128);
        monitor.execute("assemble addi x1,x0,1").unwrap();
        monitor.execute("view 0x20").unwrap();
        monitor.execute("mark entry").unwrap();
        monitor.execute("view 0x30").unwrap();
        monitor.execute("jump @entry").unwrap();
        assert_eq!(monitor.view_address, 0x20);
        assert_eq!(monitor.machine.pc, 0);
        assert!(
            monitor
                .execute("memory @entry 1")
                .unwrap()
                .contains("0x0000000000000020")
        );
        assert!(
            monitor
                .execute("marks")
                .unwrap()
                .contains("@entry=0x0000000000000020")
        );
    }

    #[test]
    fn explicit_marks_can_be_removed_and_invalid_names_are_rejected() {
        let mut monitor = Monitor::new(128);
        monitor.execute("mark code 0x10").unwrap();
        assert!(
            monitor
                .execute("unmark @code")
                .unwrap()
                .contains("unmarked @code")
        );
        assert_eq!(monitor.execute("marks").unwrap(), "marks: none");
        let error = monitor.execute("mark 1bad 0x10").unwrap_err();
        assert_eq!(error.code, "MON-MARK-006");
        assert_eq!(
            monitor.execute("jump @code").unwrap_err().code,
            "MON-MARK-005"
        );
    }

    #[test]
    fn reset_removes_marks_and_memory_undo_history() {
        let mut monitor = Monitor::new(128);
        monitor.execute("mark start 0x10").unwrap();
        monitor.execute("edit 0x10 aa").unwrap();
        monitor.execute("reset").unwrap();
        assert_eq!(monitor.execute("marks").unwrap(), "marks: none");
        assert_eq!(monitor.execute("undo").unwrap_err().code, "MON-MEM-007");
        assert_eq!(monitor.machine.memory.load8(0x10).unwrap(), 0);
    }

    #[test]
    fn host_breakpoint_stops_before_instruction_and_continue_bypasses_it() {
        let mut monitor = Monitor::new(128);
        monitor.execute("assemble addi x1,x0,1").unwrap();
        let breakpoint = monitor.execute("break 0x0").unwrap();
        assert!(breakpoint.contains("breakpoint #1"));
        assert!(
            monitor
                .execute("run 3")
                .unwrap()
                .contains("stopped: breakpoint #1")
        );
        assert_eq!(monitor.machine.x[1], 0);

        assert!(
            monitor
                .execute("continue 1")
                .unwrap()
                .contains("ran 1 step(s)")
        );
        assert_eq!(monitor.machine.x[1], 1);
        assert!(
            monitor
                .execute("info break")
                .unwrap()
                .contains("#1 addr=0x0000000000000000")
        );
        assert!(
            monitor
                .execute("delete 1")
                .unwrap()
                .contains("breakpoint #1 deleted")
        );
    }

    #[test]
    fn write_watchpoint_stops_after_store_and_reports_access() {
        let mut monitor = Monitor::new(128);
        monitor.execute("assemble sw x2,0(x1)").unwrap();
        monitor.machine.x[1] = 0x10;
        monitor.machine.x[2] = 0x1122_3344;
        monitor.execute("watch 0x10 4").unwrap();

        let stopped = monitor.execute("run 1").unwrap();
        assert!(stopped.contains("stopped: watchpoint #1"));
        assert!(stopped.contains("write addr=0x0000000000000010 width=4"));
        assert_eq!(monitor.machine.memory.load32(0x10).unwrap(), 0x1122_3344);
    }

    #[test]
    fn read_and_access_watchpoints_are_listed_and_deleted_separately() {
        let mut monitor = Monitor::new(128);
        monitor.execute("assemble lw x3,0(x1)").unwrap();
        monitor.machine.x[1] = 0x10;
        monitor.machine.memory.store32(0x10, 0x8000_0001).unwrap();
        monitor.execute("rwatch 0x10 4").unwrap();
        monitor.execute("awatch 0x20").unwrap();

        let info = monitor.execute("info watch").unwrap();
        assert!(info.contains("#1 read addr=0x0000000000000010 width=4"));
        assert!(info.contains("#2 access addr=0x0000000000000020 width=1"));
        let stopped = monitor.execute("run 1").unwrap();
        assert!(stopped.contains("read addr=0x0000000000000010 width=4"));
        assert_eq!(monitor.machine.x[3], 0xffff_ffff_8000_0001);
        monitor.execute("delete watch 1").unwrap();
        assert!(!monitor.execute("info watch").unwrap().contains("#1 read"));
    }

    #[test]
    fn program_symbols_location_and_inferred_call_stack_are_visible() {
        let mut monitor = Monitor::new(128);
        let loaded = monitor
            .assemble_program("_start: jal ra,func\n        addi x2,x0,9\nfunc:   addi x3,x0,7")
            .unwrap();
        assert!(loaded.contains("2 symbol(s)"));
        assert!(monitor.execute("symbols").unwrap().contains("func"));

        monitor.execute("step").unwrap();
        assert!(monitor.execute("where").unwrap().contains("func"));
        assert!(monitor.execute("stack").unwrap().contains("target=func"));
        assert!(monitor.execute("history").unwrap().contains("jal x1,func"));
    }

    #[test]
    fn execution_history_is_bounded_and_fifo() {
        let mut monitor = Monitor::new(128);
        monitor.assemble_program("_start: jal x0,0").unwrap();
        monitor.execute("run 4100").unwrap();
        assert_eq!(monitor.history.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(monitor.history.first().unwrap().sequence, 5);
        assert!(monitor.execute("history 2").unwrap().contains("#004100"));
    }

    #[test]
    fn source_address_commands_reject_unknown_symbols() {
        let mut monitor = Monitor::new(128);
        monitor.assemble_program("_start: addi x1,x0,1").unwrap();
        let error = monitor.execute("jump @missing").unwrap_err();
        assert_eq!(error.code, "MON-MARK-005");
        assert_eq!(
            monitor.execute("symbols").unwrap(),
            "symbols:\n  0x0000000000000000 _start"
        );
    }

    #[test]
    fn snapshot_roundtrip_restores_machine_and_debug_state() {
        let mut monitor = Monitor::new(128);
        monitor.assemble_program("_start: addi x1,x0,1").unwrap();
        monitor.execute("mark entry 0x0").unwrap();
        monitor.execute("break 0x0").unwrap();
        monitor.execute("watch 0x10 4").unwrap();
        monitor.machine.x[1] = 0x8000_0000;
        monitor.machine.memory.store32(0x10, 0x1122_3344).unwrap();
        let snapshot = monitor.snapshot_bytes().unwrap();

        monitor.machine.x[1] = 99;
        monitor.machine.memory.store32(0x10, 0).unwrap();
        monitor.execute("reset").unwrap();
        monitor.restore_snapshot_bytes(&snapshot).unwrap();

        assert_eq!(monitor.machine.x[1], 0x8000_0000);
        assert_eq!(monitor.machine.memory.load32(0x10).unwrap(), 0x1122_3344);
        assert!(monitor.execute("symbols").unwrap().contains("_start"));
        assert!(monitor.execute("marks").unwrap().contains("@entry"));
        assert!(monitor.execute("info break").unwrap().contains("#1"));
        assert!(monitor.execute("info watch").unwrap().contains("#1"));
    }

    #[test]
    fn malformed_snapshot_is_rejected_without_mutating_state() {
        let mut monitor = Monitor::new(128);
        monitor.assemble("addi x1,x0,1").unwrap();
        monitor.machine.x[1] = 7;
        let snapshot = monitor.snapshot_bytes().unwrap();
        let truncated = &snapshot[..snapshot.len() - 1];
        let error = monitor.restore_snapshot_bytes(truncated).unwrap_err();
        assert_eq!(error.code, "MON-PERSIST-012");
        assert_eq!(monitor.machine.x[1], 7);
    }

    #[test]
    fn project_file_roundtrip_restores_source_and_state() {
        let path = std::env::temp_dir().join(format!(
            "rvmonitor-project-{}-{}.rvp",
            std::process::id(),
            234_567u64
        ));
        let path_text = path.to_string_lossy().into_owned();
        let mut monitor = Monitor::new(128);
        monitor
            .assemble_program("_start: addi x1,x0,1\ndone: addi x2,x0,2")
            .unwrap();
        monitor
            .execute(&format!("project-save {path_text}"))
            .unwrap();
        monitor.execute("reset").unwrap();
        monitor
            .execute(&format!("project-load {path_text}"))
            .unwrap();
        assert!(monitor.execute("symbols").unwrap().contains("done"));
        assert!(monitor.execute("where").unwrap().contains("_start"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn backend_console_runs_common_commands_against_machine_backend() {
        let mut console = BackendConsole::new(Machine::new(128));
        assert!(
            console
                .execute("assemble addi x1,x0,1")
                .unwrap()
                .contains("loaded 4 byte(s)")
        );
        assert_eq!(
            console.execute("break 0").unwrap(),
            "breakpoint #1 set at 0x0000000000000000"
        );
        assert!(console.execute("run 1").unwrap().contains("breakpoint #1"));
        assert!(console.execute("info break").unwrap().contains("#1"));
        assert!(
            console
                .execute("continue 1")
                .unwrap()
                .contains("ran 1 step(s)")
        );
        assert!(
            console
                .execute("regs")
                .unwrap()
                .contains("x01=0x0000000000000001")
        );
        assert_eq!(
            console.execute("delete 1").unwrap(),
            "breakpoint #1 deleted"
        );

        let original = console.execute("memory 0 4").unwrap();
        console.execute("edit 0 13 00 20 00").unwrap();
        console.execute("undo").unwrap();
        assert_eq!(console.execute("memory 0 4").unwrap(), original);

        let mut memory_console = BackendConsole::new(Machine::new(128));
        memory_console.execute("assemble lw x1,8(x0)").unwrap();
        memory_console.execute("edit 8 44 33 22 11").unwrap();
        assert!(
            memory_console
                .execute("rwatch 8 4")
                .unwrap()
                .contains("watchpoint #1")
        );
        assert!(
            memory_console
                .execute("run 1")
                .unwrap()
                .contains("watchpoint #1")
        );
        assert!(
            memory_console
                .execute("history")
                .unwrap()
                .contains("read 0x0000000000000008/4")
        );
        assert!(
            memory_console
                .execute("info watch")
                .unwrap()
                .contains("#1 read")
        );
        assert_eq!(
            memory_console.execute("delete watch 1").unwrap(),
            "watchpoint #1 deleted"
        );

        let mut symbol_console = BackendConsole::new(Machine::new(128));
        symbol_console
            .execute("assemble-program _start: jal x0,func\nfunc: addi x1,x0,1")
            .unwrap();
        assert!(symbol_console.execute("symbols").unwrap().contains("func"));
        assert!(
            symbol_console
                .execute("disasm 0 2")
                .unwrap()
                .contains("jal x0,func")
        );
        assert!(symbol_console.execute("where").unwrap().contains("_start"));
        symbol_console.execute("step").unwrap();
        assert!(
            symbol_console
                .execute("history")
                .unwrap()
                .contains("jal x0,func")
        );
    }
}
