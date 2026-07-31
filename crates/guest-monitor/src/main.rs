#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

use luna_isa_core::{ADDI_MASK, ADDI_MATCH, GENERATED_OPCODES};
use luna_target_api::Breakpoint;
use luna_target_api::StopReason;
use luna_target_api::TargetCapabilities;
use luna_target_api::TargetContext;

const UART_BASE: usize = 0x1000_0000;
const UART_LSR: usize = 5;
const UART_LSR_DATA_READY: u8 = 1 << 0;
const UART_LSR_EMPTY: u8 = 1 << 5;
const COMMAND_CAPACITY: usize = 96;
const TARGET_RAM_START: u64 = 0x8000_0000;
const TARGET_RAM_END: u64 = 0x8002_0000;
const EBREAK_WORD: u32 = 0x0010_0073;
const MAX_PERMANENT_BREAKPOINTS: usize = 4;
const MAX_MEMORY_DUMP: u64 = 128;
const MAX_EDIT_BYTES: usize = 32;
const MAX_SOURCE_LINES: usize = 16;
const MAX_SYMBOLS: usize = 8;
const SYMBOL_NAME_CAPACITY: usize = 16;

global_asm!(include_str!("entry.S"));

static mut CONTEXT: TargetContext = TargetContext::empty();
static mut TEMPORARY_BREAKPOINT: Breakpoint = Breakpoint::disabled();
static mut PERMANENT_BREAKPOINTS: [Breakpoint; MAX_PERMANENT_BREAKPOINTS] =
    [Breakpoint::disabled(); MAX_PERMANENT_BREAKPOINTS];
static mut STEPPED_PERMANENT_BREAKPOINT: u8 = u8::MAX;
static mut SYMBOLS: [GuestSymbol; MAX_SYMBOLS] = [GuestSymbol::empty(); MAX_SYMBOLS];
static mut MEMORY_UNDO: MemoryUndo = MemoryUndo::empty();
static TARGET_STACK: [u8; 8192] = [0; 8192];

#[derive(Clone, Copy)]
struct GuestSymbol {
    name: [u8; SYMBOL_NAME_CAPACITY],
    length: usize,
    address: u64,
    enabled: bool,
}

impl GuestSymbol {
    const fn empty() -> Self {
        Self {
            name: [0; SYMBOL_NAME_CAPACITY],
            length: 0,
            address: 0,
            enabled: false,
        }
    }
}

#[derive(Clone, Copy)]
struct MemoryUndo {
    address: u64,
    length: usize,
    original: [u8; MAX_EDIT_BYTES],
    edited: [u8; MAX_EDIT_BYTES],
    valid: bool,
}

impl MemoryUndo {
    const fn empty() -> Self {
        Self {
            address: 0,
            length: 0,
            original: [0; MAX_EDIT_BYTES],
            edited: [0; MAX_EDIT_BYTES],
            valid: false,
        }
    }
}

#[derive(Clone, Copy)]
enum MemoryWriteError {
    Overflow,
    OutsideRam,
    ActiveBreakpoint,
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    uart_write("\r\nRVMonitor panic\r\n");
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    let context = core::ptr::addr_of_mut!(CONTEXT) as usize;
    let trap = trap_entry as *const () as usize;
    unsafe {
        install_trap(trap, context);
    }
    uart_write("\r\nRVMonitor 4B M-mode\r\n");
    uart_write("target: RV64 ILP32D U-mode, hart=1, C=off\r\n");
    let capabilities = TargetCapabilities::RV64_BARE_METAL_V1;
    if capabilities.xlen == 64 && capabilities.flen == 64 {
        uart_write("capabilities: I M F D Zicsr Zifencei\r\n");
    }
    uart_write("target workspace: 0x");
    uart_hex(target_workspace_start());
    uart_write("..0x");
    uart_hex(target_workspace_end());
    uart_write("\r\n");
    uart_write("target: entering U-mode\r\n");
    let target_stack = TARGET_STACK.as_ptr() as usize + TARGET_STACK.len();
    unsafe {
        enter_user(target_entry as *const () as usize, target_stack);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_trap(context: *mut TargetContext) -> ! {
    let context = unsafe { &mut *context };
    restore_temporary_breakpoint();
    restore_stepped_permanent_breakpoint();
    uart_write("trap: ");
    if context.mcause == StopReason::Breakpoint as u64 {
        if let Some(slot) = permanent_breakpoint_at(context.mepc) {
            uart_write("breakpoint #");
            uart_decimal((slot + 1) as u64);
        } else {
            uart_write("breakpoint");
        }
    } else {
        uart_write("mcause=0x");
        uart_hex(context.mcause);
    }
    uart_write(" pc=0x");
    uart_hex(context.mepc);
    uart_write("\r\n");
    monitor_loop(context as *const TargetContext as *mut TargetContext);
}

fn monitor_loop(context: *mut TargetContext) -> ! {
    let mut line = [0u8; COMMAND_CAPACITY];
    loop {
        uart_write("rvmonitor> ");
        let length = uart_read_line(&mut line);
        match &line[..length] {
            b"help" | b"?" => print_help(),
            b"regs" | b"registers" => print_registers(context),
            command if command.starts_with(b"setf ") => set_float_register(context, &command[5..]),
            command if command.starts_with(b"memory ") => print_memory(&command[7..]),
            command if command.starts_with(b"edit ") => edit_memory(context, &command[5..]),
            command if command.starts_with(b"data ") => data_directive(context, &command[5..]),
            b"undo" => undo_memory(context),
            command if command.starts_with(b"assemble ") => {
                assemble_command(context, &command[9..])
            }
            command if command.starts_with(b"assemble-program ") => {
                assemble_program_command(context, &command[17..])
            }
            b"symbols" => print_symbols(),
            command if command.starts_with(b"disasm ") => print_disassembly(&command[7..]),
            b"step" | b"s" => step_target(context),
            b"continue" | b"c" => continue_target(context),
            command if command.starts_with(b"break ") => break_target(&command[6..]),
            command if command.starts_with(b"delete ") => delete_breakpoint(&command[7..]),
            b"info break" | b"info b" => print_breakpoints(),
            b"quit" | b"exit" | b"q" => {
                uart_write("bye\r\n");
            }
            b"" => {}
            _ => uart_write("error: unknown command; use help\r\n"),
        }
    }
}

fn print_help() {
    uart_write(
        "help/? regs/registers setf <freg> <hex64> memory <addr> <length> edit <addr> <hex-bytes> data <addr> <directive> <bits> undo assemble <addr> <instruction> assemble-program <addr> ... end symbols disasm <addr|label> <count> step/s continue/c break <addr|label> delete <n> info break quit/q\r\n",
    );
}

fn print_registers(context: *mut TargetContext) {
    let context = unsafe { &*context };
    uart_write("pc=0x");
    uart_hex(context.pc);
    uart_write(" mepc=0x");
    uart_hex(context.mepc);
    uart_write(" mcause=0x");
    uart_hex(context.mcause);
    uart_write(" mtval=0x");
    uart_hex(context.mtval);
    uart_write("\r\n");
    uart_write("mstatus=0x");
    uart_hex(context.mstatus);
    uart_write(" fcsr=0x");
    uart_hex(u64::from(context.fcsr));
    uart_write("\r\n");

    uart_write("integer registers:\r\n");
    for index in 0..32 {
        uart_write("x");
        uart_decimal(index as u64);
        uart_write("=0x");
        uart_hex(context.x[index]);
        if index % 4 == 3 {
            uart_write("\r\n");
        } else {
            uart_write("  ");
        }
    }

    uart_write("floating registers (raw bits):\r\n");
    for index in 0..32 {
        uart_write("f");
        uart_decimal(index as u64);
        uart_write("=0x");
        uart_hex(context.f[index]);
        if index % 4 == 3 {
            uart_write("\r\n");
        } else {
            uart_write("  ");
        }
    }
}

fn set_float_register(context: *mut TargetContext, argument: &[u8]) {
    if unsafe { (*context).mcause } != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        return;
    }
    let Some((register_bytes, value_bytes)) = split_token_space(argument) else {
        uart_write("error: setf expects <freg> <hex64>\r\n");
        return;
    };
    let Some(register) = parse_float_register(register_bytes) else {
        uart_write("error: setf register must be f0..f31\r\n");
        return;
    };
    let Some(value) = parse_hex(value_bytes) else {
        uart_write("error: setf value must be a hexadecimal 64-bit pattern\r\n");
        return;
    };
    unsafe {
        (*context).f[usize::from(register)] = value;
    }
    uart_write("set f");
    uart_decimal(u64::from(register));
    uart_write("=0x");
    uart_hex(value);
    uart_write("\r\n");
}

fn edit_memory(context: *mut TargetContext, argument: &[u8]) {
    if unsafe { (*context).mcause } != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        return;
    }
    let Some((address_bytes, data_bytes)) = split_token_space(argument) else {
        uart_write("error: edit expects <hex-address> <hex-bytes>\r\n");
        return;
    };
    let Some(address) = parse_hex(address_bytes) else {
        uart_write("error: edit address must be hexadecimal\r\n");
        return;
    };
    let mut edited = [0u8; MAX_EDIT_BYTES];
    let Some(length) = parse_hex_bytes(data_bytes, &mut edited) else {
        uart_write("error: edit expects 1..32 complete hexadecimal bytes\r\n");
        return;
    };
    if let Err(error) = write_memory_transaction(address, edited, length) {
        match error {
            MemoryWriteError::Overflow => uart_write("error: edit range overflows\r\n"),
            MemoryWriteError::OutsideRam => {
                uart_write("error: edit range is outside target RAM\r\n")
            }
            MemoryWriteError::ActiveBreakpoint => {
                uart_write("error: edit overlaps an active breakpoint\r\n")
            }
        }
        return;
    }
    uart_write("edited ");
    uart_decimal(length as u64);
    uart_write(" byte(s) at 0x");
    uart_hex(address);
    uart_write("\r\n");
}

fn data_directive(context: *mut TargetContext, argument: &[u8]) {
    if unsafe { (*context).mcause } != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        return;
    }
    let Some((address_bytes, rest)) = split_token_space(argument) else {
        uart_write("error: data expects <hex-address> <directive> <bits>\r\n");
        return;
    };
    let Some((directive, value_bytes)) = split_token_space(rest) else {
        uart_write("error: data expects <hex-address> <directive> <bits>\r\n");
        return;
    };
    let Some(address) = parse_hex(address_bytes) else {
        uart_write("error: data address must be hexadecimal\r\n");
        return;
    };
    let mut data = [0u8; MAX_EDIT_BYTES];
    let Some(length) = encode_data_directive(directive, value_bytes, &mut data) else {
        uart_write("error: unsupported directive or value; use exact integer/IEEE bits\r\n");
        return;
    };
    if let Err(error) = write_memory_transaction(address, data, length) {
        match error {
            MemoryWriteError::Overflow => uart_write("error: data range overflows\r\n"),
            MemoryWriteError::OutsideRam => {
                uart_write("error: data range is outside target RAM\r\n")
            }
            MemoryWriteError::ActiveBreakpoint => {
                uart_write("error: data overlaps an active breakpoint\r\n")
            }
        }
        return;
    }
    uart_write("stored ");
    uart_bytes(directive);
    uart_write(" at 0x");
    uart_hex(address);
    uart_write(" (");
    uart_decimal(length as u64);
    uart_write(" byte(s))\r\n");
}

fn encode_data_directive(
    directive: &[u8],
    value: &[u8],
    destination: &mut [u8; MAX_EDIT_BYTES],
) -> Option<usize> {
    let width = match directive {
        b".byte" => 1,
        b".half" | b".binary16" => 2,
        b".word" | b".float" => 4,
        b".dword" | b".double" => 8,
        b".binary128" => 16,
        _ => return None,
    };
    if width == 16 {
        let length = parse_hex_bytes(value, destination)?;
        if length != width {
            return None;
        }
        destination[..width].reverse();
        return Some(length);
    }
    let number = parse_hex(value).or_else(|| parse_decimal(value))?;
    let limit = if width == 8 {
        u64::MAX
    } else {
        (1u64 << (width * 8)) - 1
    };
    if number > limit {
        return None;
    }
    for index in 0..width {
        destination[index] = (number >> (index * 8)) as u8;
    }
    Some(width)
}

fn write_memory_transaction(
    address: u64,
    edited: [u8; MAX_EDIT_BYTES],
    length: usize,
) -> Result<(), MemoryWriteError> {
    let Some(end) = address.checked_add(length as u64) else {
        return Err(MemoryWriteError::Overflow);
    };
    if address < TARGET_RAM_START || end > TARGET_RAM_END {
        return Err(MemoryWriteError::OutsideRam);
    }
    for offset in 0..length {
        let byte_address = address + offset as u64;
        if permanent_breakpoint_at(byte_address & !3).is_some()
            || temporary_breakpoint_at(byte_address & !3)
        {
            return Err(MemoryWriteError::ActiveBreakpoint);
        }
    }
    let mut original = [0u8; MAX_EDIT_BYTES];
    for offset in 0..length {
        original[offset] =
            unsafe { core::ptr::read_volatile((address + offset as u64) as *const u8) };
    }
    for offset in 0..length {
        unsafe {
            core::ptr::write_volatile((address + offset as u64) as *mut u8, edited[offset]);
        }
    }
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(MEMORY_UNDO),
            MemoryUndo {
                address,
                length,
                original,
                edited,
                valid: true,
            },
        );
    }
    flush_icache();
    Ok(())
}

fn undo_memory(context: *mut TargetContext) {
    if unsafe { (*context).mcause } != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        return;
    }
    let undo = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(MEMORY_UNDO)) };
    if !undo.valid {
        uart_write("error: no memory edit to undo\r\n");
        return;
    }
    for offset in 0..undo.length {
        let current =
            unsafe { core::ptr::read_volatile((undo.address + offset as u64) as *const u8) };
        if current != undo.edited[offset] {
            clear_memory_undo();
            uart_write("error: edited memory changed; undo refused\r\n");
            return;
        }
    }
    for offset in 0..undo.length {
        unsafe {
            core::ptr::write_volatile(
                (undo.address + offset as u64) as *mut u8,
                undo.original[offset],
            );
        }
    }
    clear_memory_undo();
    flush_icache();
    uart_write("undone ");
    uart_decimal(undo.length as u64);
    uart_write(" byte(s) at 0x");
    uart_hex(undo.address);
    uart_write("\r\n");
}

fn clear_memory_undo() {
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(MEMORY_UNDO), MemoryUndo::empty());
    }
}

fn print_memory(argument: &[u8]) {
    let Some((address, length)) = parse_memory_range(argument) else {
        uart_write("error: memory expects <hex-address> <decimal-length>\r\n");
        return;
    };
    if length == 0 || length > MAX_MEMORY_DUMP {
        uart_write("error: memory length must be between 1 and 128\r\n");
        return;
    }
    let Some(end) = address.checked_add(length) else {
        uart_write("error: memory range overflows\r\n");
        return;
    };
    if address < TARGET_RAM_START || end > TARGET_RAM_END {
        uart_write("error: memory range is outside target RAM\r\n");
        return;
    }

    let mut offset = 0u64;
    while offset < length {
        let row_length = core::cmp::min(16, length - offset);
        let row_address = address + offset;
        uart_write("0x");
        uart_hex(row_address);
        uart_write(": ");
        let mut column = 0u64;
        while column < 16 {
            if column < row_length {
                uart_hex_byte(unsafe {
                    core::ptr::read_volatile((row_address + column) as *const u8)
                });
            } else {
                uart_write("  ");
            }
            uart_put(b' ');
            column += 1;
        }
        uart_write("|");
        column = 0;
        while column < row_length {
            let byte = unsafe { core::ptr::read_volatile((row_address + column) as *const u8) };
            uart_put(if (32..=126).contains(&byte) {
                byte
            } else {
                b'.'
            });
            column += 1;
        }
        uart_write("|\r\n");
        offset += row_length;
    }
}

fn parse_memory_range(argument: &[u8]) -> Option<(u64, u64)> {
    let separator = argument.iter().position(|byte| *byte == b' ')?;
    let address = parse_hex(&argument[..separator])?;
    let length = parse_decimal(argument[separator + 1..].trim_ascii())?;
    Some((address, length))
}

fn print_symbols() {
    uart_write("symbols:\r\n");
    let mut found = false;
    for index in 0..MAX_SYMBOLS {
        let symbol = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SYMBOLS[index])) };
        if symbol.enabled {
            found = true;
            uart_write("  0x");
            uart_hex(symbol.address);
            uart_write(" ");
            uart_bytes(&symbol.name[..symbol.length]);
            uart_write("\r\n");
        }
    }
    if !found {
        uart_write("  none\r\n");
    }
}

fn print_disassembly(argument: &[u8]) {
    let Some((address, count)) = split_once_address_space(argument) else {
        uart_write("error: disasm expects <address|label> <decimal-count>\r\n");
        return;
    };
    if count == 0 || count > 16 {
        uart_write("error: disasm count must be between 1 and 16\r\n");
        return;
    }
    let Some(end) = address.checked_add(count * 4) else {
        uart_write("error: disasm range overflows\r\n");
        return;
    };
    if address % 4 != 0 || address < TARGET_RAM_START || end > TARGET_RAM_END {
        uart_write("error: disasm range is outside target RAM\r\n");
        return;
    }
    for index in 0..count {
        let instruction_address = address + index * 4;
        let Some(word) = target_load32(instruction_address) else {
            uart_write("error: cannot read disassembly word\r\n");
            return;
        };
        uart_write("0x");
        uart_hex(instruction_address);
        uart_write(": ");
        uart_hex(u64::from(word));
        if let Some(symbol) = symbol_at(instruction_address) {
            uart_write(" <");
            uart_bytes(&symbol.name[..symbol.length]);
            uart_write(">");
        }
        uart_write("  ");
        print_disassembled_word(instruction_address, word);
        uart_write("\r\n");
    }
}

fn print_disassembled_word(address: u64, word: u32) {
    if word == EBREAK_WORD {
        uart_write("ebreak");
        return;
    }
    if word & ADDI_MASK == ADDI_MATCH {
        let rd = ((word >> 7) & 31) as u8;
        let rs1 = ((word >> 15) & 31) as u8;
        let immediate = (word as i32 >> 20) as i16;
        uart_write("addi x");
        uart_decimal(u64::from(rd));
        uart_write(",x");
        uart_decimal(u64::from(rs1));
        uart_write(",");
        uart_signed_decimal(immediate);
        return;
    }
    for opcode in GENERATED_OPCODES {
        if (opcode.mnemonic == "fadd.s" || opcode.mnemonic == "fadd.d")
            && word & opcode.mask == opcode.match_value
        {
            uart_write(opcode.mnemonic);
            uart_write(" f");
            uart_decimal(u64::from((word >> 7) & 31));
            uart_write(",f");
            uart_decimal(u64::from((word >> 15) & 31));
            uart_write(",f");
            uart_decimal(u64::from((word >> 20) & 31));
            let rm = (word >> 12) & 7;
            if rm != 0 {
                uart_write(",");
                uart_decimal(u64::from(rm));
            }
            return;
        }
    }
    if word & 0x7f == 0x63 {
        let mnemonic = match (word >> 12) & 0x7 {
            0b000 => "beq",
            0b001 => "bne",
            _ => "",
        };
        if !mnemonic.is_empty() {
            let rs1 = ((word >> 15) & 31) as u8;
            let rs2 = ((word >> 20) & 31) as u8;
            let immediate = (((word >> 31) & 1) << 12)
                | (((word >> 25) & 0x3f) << 5)
                | (((word >> 8) & 0xf) << 1)
                | (((word >> 7) & 1) << 11);
            let immediate = ((immediate as i32) << 19 >> 19) as i16;
            uart_write(mnemonic);
            uart_write(" x");
            uart_decimal(u64::from(rs1));
            uart_write(",x");
            uart_decimal(u64::from(rs2));
            uart_write(",");
            let target = address.wrapping_add_signed(i64::from(immediate));
            if let Some(symbol) = symbol_at(target) {
                uart_bytes(&symbol.name[..symbol.length]);
            } else {
                uart_signed_decimal(immediate);
            }
            return;
        }
    }
    if word & 0x7f == 0x6f {
        let rd = ((word >> 7) & 31) as u8;
        let immediate = (((word >> 31) & 1) << 20)
            | (((word >> 21) & 0x3ff) << 1)
            | (((word >> 20) & 1) << 11)
            | (((word >> 12) & 0xff) << 12);
        let immediate = ((immediate as i32) << 11 >> 11) as i32;
        uart_write("jal x");
        uart_decimal(u64::from(rd));
        uart_write(",");
        let target = address.wrapping_add_signed(i64::from(immediate));
        if let Some(symbol) = symbol_at(target) {
            uart_bytes(&symbol.name[..symbol.length]);
        } else {
            uart_signed_decimal(i16::try_from(immediate).unwrap_or(0));
        }
        return;
    }
    if word & 0x7f == 0x67 && (word >> 12) & 0x7 == 0 {
        let rd = ((word >> 7) & 31) as u8;
        let rs1 = ((word >> 15) & 31) as u8;
        let immediate = (word as i32 >> 20) as i16;
        uart_write("jalr x");
        uart_decimal(u64::from(rd));
        uart_write(",");
        uart_signed_decimal(immediate);
        uart_write("(x");
        uart_decimal(u64::from(rs1));
        uart_write(")");
        return;
    }
    for mnemonic in ["ld", "sd"] {
        for opcode in GENERATED_OPCODES {
            if opcode.mnemonic != mnemonic || word & opcode.mask != opcode.match_value {
                continue;
            }
            let immediate = if mnemonic == "ld" {
                (word as i32 >> 20) as i16
            } else {
                let encoded = (((word >> 25) & 0x7f) << 5) | ((word >> 7) & 0x1f);
                ((encoded as i32) << 20 >> 20) as i16
            };
            uart_write(mnemonic);
            uart_write(" x");
            if mnemonic == "ld" {
                uart_decimal(u64::from((word >> 7) & 31));
                uart_write(",");
            } else {
                uart_decimal(u64::from((word >> 20) & 31));
                uart_write(",");
            }
            uart_signed_decimal(immediate);
            uart_write("(x");
            uart_decimal(u64::from((word >> 15) & 31));
            uart_write(")");
            return;
        }
    }
    for opcode in GENERATED_OPCODES {
        if opcode.extension == "rv_c" || opcode.extension == "rv64_c" {
            continue;
        }
        if word & opcode.mask == opcode.match_value {
            uart_write(opcode.mnemonic);
            return;
        }
    }
    uart_write(".word 0x");
    uart_hex(u64::from(word));
}

fn symbol_at(address: u64) -> Option<GuestSymbol> {
    for index in 0..MAX_SYMBOLS {
        let symbol = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SYMBOLS[index])) };
        if symbol.enabled && symbol.address == address {
            return Some(symbol);
        }
    }
    None
}

fn assemble_command(context: *mut TargetContext, argument: &[u8]) {
    let Some((address, source)) = split_once_space(argument) else {
        uart_write("error: assemble expects <address> <instruction>\r\n");
        return;
    };
    if !valid_target_program_word_address(address) {
        uart_write("error: assemble address must be an aligned target workspace word\r\n");
        return;
    }
    if permanent_breakpoint_at(address).is_some() || temporary_breakpoint_at(address) {
        uart_write("error: cannot assemble over an active breakpoint\r\n");
        return;
    }
    let empty_symbols = [GuestSymbol::empty(); MAX_SYMBOLS];
    let Some(word) = parse_source_instruction(source, address, &empty_symbols) else {
        uart_write("error: expected supported instruction with valid operands\r\n");
        return;
    };
    clear_memory_undo();
    if !target_store32(address, word) {
        uart_write("error: cannot write assembled instruction\r\n");
        return;
    }
    flush_icache();
    let context = unsafe { &mut *context };
    context.pc = address;
    context.mepc = address;
    context.mcause = StopReason::Breakpoint as u64;
    context.mtval = 0;
    uart_write("assembled instruction at 0x");
    uart_hex(address);
    uart_write(" = 0x");
    uart_hex(u64::from(word));
    uart_write("\r\n");
}

fn assemble_program_command(context: *mut TargetContext, argument: &[u8]) {
    let Some(address) = parse_hex(argument.trim_ascii()) else {
        uart_write("error: assemble-program expects a hexadecimal address\r\n");
        return;
    };
    if !valid_target_program_word_address(address) {
        uart_write("error: assemble-program address must be an aligned target workspace word\r\n");
        return;
    }

    uart_write(
        "source mode: enter integer/control, ld/sd or fadd.s/fadd.d lines, finish with end\r\n",
    );
    let mut lines = [[0u8; COMMAND_CAPACITY]; MAX_SOURCE_LINES];
    let mut lengths = [0usize; MAX_SOURCE_LINES];
    let mut count = 0usize;
    let mut overflow = false;
    let mut input = [0u8; COMMAND_CAPACITY];
    loop {
        uart_write("source> ");
        let length = uart_read_line(&mut input);
        let line = &input[..length];
        if line == b"end" {
            break;
        }
        if line.is_empty() {
            continue;
        }
        if count == MAX_SOURCE_LINES {
            overflow = true;
            continue;
        }
        lines[count][..length].copy_from_slice(line);
        lengths[count] = length;
        count += 1;
    }

    if overflow {
        uart_write("error: source program exceeds 16 instruction lines\r\n");
        return;
    }
    if count == 0 {
        uart_write("error: source program is empty\r\n");
        return;
    }
    let mut staged_symbols = [GuestSymbol::empty(); MAX_SYMBOLS];
    let mut instruction_count = 0usize;
    for index in 0..count {
        let line = &lines[index][..lengths[index]];
        if let Some(label) = line.strip_suffix(b":") {
            let line_address = address + (instruction_count as u64) * 4;
            let Some(symbol) = make_symbol(label, line_address) else {
                uart_write("error: invalid, duplicate or too many source labels\r\n");
                return;
            };
            if staged_symbols
                .iter()
                .any(|slot| slot.enabled && &slot.name[..slot.length] == label)
            {
                uart_write("error: invalid, duplicate or too many source labels\r\n");
                return;
            }
            if let Some(slot) = staged_symbols.iter_mut().find(|slot| !slot.enabled) {
                *slot = symbol;
            } else {
                uart_write("error: invalid, duplicate or too many source labels\r\n");
                return;
            }
        } else {
            instruction_count += 1;
        }
    }
    if instruction_count == 0 {
        uart_write("error: source program contains no instructions\r\n");
        return;
    }

    let Some(end_address) = address.checked_add((instruction_count as u64) * 4) else {
        uart_write("error: source program address overflows\r\n");
        return;
    };
    if end_address > TARGET_RAM_END {
        uart_write("error: source program does not fit in target RAM\r\n");
        return;
    }

    let mut words = [0u32; MAX_SOURCE_LINES];
    let mut word_addresses = [0u64; MAX_SOURCE_LINES];
    let mut word_count = 0usize;
    for index in 0..count {
        let line = &lines[index][..lengths[index]];
        if line.ends_with(b":") {
            continue;
        }
        let line_address = address + (word_count as u64) * 4;
        if !valid_target_program_word_address(line_address) {
            uart_write("error: source program exceeds target workspace\r\n");
            return;
        }
        let Some(word) = parse_source_instruction(line, line_address, &staged_symbols) else {
            uart_write(
                "error: source line supports integer/control, ld/sd or fadd.s/fadd.d syntax\r\n",
            );
            return;
        };
        if permanent_breakpoint_at(line_address).is_some() || temporary_breakpoint_at(line_address)
        {
            uart_write("error: source overlaps an active breakpoint\r\n");
            return;
        }
        words[word_count] = word;
        word_addresses[word_count] = line_address;
        word_count += 1;
    }

    clear_memory_undo();
    for index in 0..word_count {
        if !target_store32(word_addresses[index], words[index]) {
            uart_write("error: cannot write assembled source program\r\n");
            return;
        }
    }
    flush_icache();
    let context = unsafe { &mut *context };
    context.pc = address;
    context.mepc = address;
    context.mcause = StopReason::Breakpoint as u64;
    context.mtval = 0;
    unsafe {
        for (index, symbol) in staged_symbols.iter().enumerate() {
            core::ptr::write_volatile(core::ptr::addr_of_mut!(SYMBOLS[index]), *symbol);
        }
    }
    uart_write("assembled program: ");
    uart_decimal(word_count as u64);
    uart_write(" instruction(s) at 0x");
    uart_hex(address);
    uart_write("\r\n");
}

fn parse_source_instruction(
    source: &[u8],
    address: u64,
    symbols: &[GuestSymbol; MAX_SYMBOLS],
) -> Option<u32> {
    if let Some(operands) = source.strip_prefix(b"addi ") {
        return parse_addi_operands(operands, symbols);
    }
    if let Some(operands) = source.strip_prefix(b"beq ") {
        return parse_branch_operands("beq", operands, address, symbols);
    }
    if let Some(operands) = source.strip_prefix(b"bne ") {
        return parse_branch_operands("bne", operands, address, symbols);
    }
    if let Some(operands) = source.strip_prefix(b"jal ") {
        let (rd_bytes, target_bytes) = split_once_comma(operands)?;
        let rd = parse_register(rd_bytes.trim_ascii())?;
        let immediate = parse_relative_target(target_bytes.trim_ascii(), address, symbols)?;
        return luna_isa_core::encode_jal(rd, immediate);
    }
    if let Some(operands) = source.strip_prefix(b"jalr ") {
        return parse_jalr_operands(operands);
    }
    if let Some(operands) = source.strip_prefix(b"ld ") {
        return parse_load_store_operands("ld", operands, symbols);
    }
    if let Some(operands) = source.strip_prefix(b"sd ") {
        return parse_load_store_operands("sd", operands, symbols);
    }
    if let Some(operands) = source.strip_prefix(b"fadd.s ") {
        return parse_fadd_operands("fadd.s", operands);
    }
    if let Some(operands) = source.strip_prefix(b"fadd.d ") {
        return parse_fadd_operands("fadd.d", operands);
    }
    None
}

fn parse_addi_operands(operands: &[u8], symbols: &[GuestSymbol; MAX_SYMBOLS]) -> Option<u32> {
    let (rd_bytes, rest) = split_once_comma(operands)?;
    let (rs1_bytes, imm_bytes) = split_once_comma(rest)?;
    let rd = parse_register(rd_bytes.trim_ascii())?;
    let rs1 = parse_register(rs1_bytes.trim_ascii())?;
    let imm = parse_signed_decimal_or_symbol(imm_bytes.trim_ascii(), symbols)?;
    luna_isa_core::encode_addi(rd, rs1, imm)
}

fn parse_branch_operands(
    mnemonic: &str,
    operands: &[u8],
    address: u64,
    symbols: &[GuestSymbol; MAX_SYMBOLS],
) -> Option<u32> {
    let (rs1_bytes, rest) = split_once_comma(operands)?;
    let (rs2_bytes, target_bytes) = split_once_comma(rest)?;
    let rs1 = parse_register(rs1_bytes.trim_ascii())?;
    let rs2 = parse_register(rs2_bytes.trim_ascii())?;
    let immediate = parse_relative_target(target_bytes.trim_ascii(), address, symbols)?;
    luna_isa_core::encode_branch(mnemonic, rs1, rs2, i16::try_from(immediate).ok()?)
}

fn parse_jalr_operands(operands: &[u8]) -> Option<u32> {
    let (rd_bytes, rest) = split_once_comma(operands)?;
    let rd = parse_register(rd_bytes.trim_ascii())?;
    let rest = rest.trim_ascii();
    let rest = rest.strip_suffix(b")")?;
    let (imm_bytes, rs1_bytes) = split_once_left_paren(rest)?;
    let immediate = parse_signed_decimal(imm_bytes.trim_ascii())?;
    let rs1 = parse_register(rs1_bytes.trim_ascii())?;
    luna_isa_core::encode_jalr(rd, rs1, immediate)
}

fn parse_load_store_operands(
    mnemonic: &str,
    operands: &[u8],
    symbols: &[GuestSymbol; MAX_SYMBOLS],
) -> Option<u32> {
    let (register_bytes, rest) = split_once_comma(operands)?;
    let rest = rest.trim_ascii().strip_suffix(b")")?;
    let (offset_bytes, base_bytes) = split_once_left_paren(rest)?;
    let offset = parse_signed_decimal_or_symbol(offset_bytes.trim_ascii(), symbols)?;
    let base = parse_register(base_bytes.trim_ascii())?;
    if mnemonic == "ld" {
        luna_isa_core::encode_load(
            mnemonic,
            parse_register(register_bytes.trim_ascii())?,
            base,
            offset,
        )
    } else {
        luna_isa_core::encode_store(
            mnemonic,
            parse_register(register_bytes.trim_ascii())?,
            base,
            offset,
        )
    }
}

fn parse_fadd_operands(mnemonic: &str, operands: &[u8]) -> Option<u32> {
    let (rd_bytes, rest) = split_once_comma(operands)?;
    let (rs1_bytes, rest) = split_once_comma(rest)?;
    let (rs2_bytes, rm_bytes) = split_once_comma(rest).unwrap_or((rest, b""));
    let rd = parse_float_register(rd_bytes.trim_ascii())?;
    let rs1 = parse_float_register(rs1_bytes.trim_ascii())?;
    let rs2 = parse_float_register(rs2_bytes.trim_ascii())?;
    let rm = if rm_bytes.is_empty() {
        0
    } else {
        let value = parse_decimal(rm_bytes.trim_ascii())?;
        (value <= 7).then_some(value as u8)?
    };
    luna_isa_core::encode_f_r(mnemonic, rd, rs1, rs2, rm)
}

fn split_once_left_paren(input: &[u8]) -> Option<(&[u8], &[u8])> {
    let separator = input.iter().position(|byte| *byte == b'(')?;
    Some((&input[..separator], &input[separator + 1..]))
}

fn split_once_space(input: &[u8]) -> Option<(u64, &[u8])> {
    let separator = input.iter().position(|byte| *byte == b' ')?;
    let address = parse_hex(&input[..separator])?;
    Some((address, input[separator + 1..].trim_ascii()))
}

fn split_token_space(input: &[u8]) -> Option<(&[u8], &[u8])> {
    let separator = input.iter().position(|byte| *byte == b' ')?;
    Some((&input[..separator], input[separator + 1..].trim_ascii()))
}

fn split_once_address_space(input: &[u8]) -> Option<(u64, u64)> {
    let separator = input.iter().position(|byte| *byte == b' ')?;
    let address = parse_address_or_symbol(&input[..separator])?;
    let count = parse_decimal(input[separator + 1..].trim_ascii())?;
    Some((address, count))
}

fn parse_address_or_symbol(input: &[u8]) -> Option<u64> {
    parse_hex(input).or_else(|| find_symbol(input))
}

fn split_once_comma(input: &[u8]) -> Option<(&[u8], &[u8])> {
    let separator = input.iter().position(|byte| *byte == b',')?;
    Some((&input[..separator], &input[separator + 1..]))
}

fn parse_register(input: &[u8]) -> Option<u8> {
    let register = input.strip_prefix(b"x")?;
    let value = parse_decimal(register)?;
    (value <= 31).then_some(value as u8)
}

fn parse_float_register(input: &[u8]) -> Option<u8> {
    let register = input.strip_prefix(b"f")?;
    let value = parse_decimal(register)?;
    (value <= 31).then_some(value as u8)
}

fn parse_signed_decimal(input: &[u8]) -> Option<i16> {
    if let Some(positive) = input.strip_prefix(b"+") {
        return parse_signed_decimal(positive);
    }
    if let Some(negative) = input.strip_prefix(b"-") {
        let value = parse_decimal(negative)?;
        return (value <= 2048).then_some(-(value as i16));
    }
    let value = parse_decimal(input)?;
    (value <= 2047).then_some(value as i16)
}

fn parse_signed_decimal_wide(input: &[u8]) -> Option<i64> {
    if let Some(positive) = input.strip_prefix(b"+") {
        return parse_signed_decimal_wide(positive);
    }
    if let Some(negative) = input.strip_prefix(b"-") {
        let value = parse_decimal(negative)?;
        return i64::try_from(value).ok()?.checked_neg();
    }
    i64::try_from(parse_decimal(input)?).ok()
}

fn parse_signed_decimal_or_symbol(
    input: &[u8],
    symbols: &[GuestSymbol; MAX_SYMBOLS],
) -> Option<i16> {
    parse_signed_decimal(input).or_else(|| {
        let address = symbols
            .iter()
            .find(|symbol| symbol.enabled && &symbol.name[..symbol.length] == input)?
            .address;
        i16::try_from(address).ok()
    })
}

fn parse_relative_target(
    input: &[u8],
    address: u64,
    symbols: &[GuestSymbol; MAX_SYMBOLS],
) -> Option<i32> {
    if let Some(immediate) = parse_signed_decimal_wide(input) {
        return i32::try_from(immediate).ok();
    }

    let (symbol_name, offset) = split_symbol_offset(input);
    let symbol_address = symbols
        .iter()
        .find(|symbol| symbol.enabled && &symbol.name[..symbol.length] == symbol_name)?
        .address;
    let absolute = (symbol_address as i64).checked_add(offset)?;
    let relative = absolute.checked_sub(i64::try_from(address).ok()?)?;
    i32::try_from(relative).ok()
}

fn split_symbol_offset(input: &[u8]) -> (&[u8], i64) {
    for (index, byte) in input.iter().enumerate().skip(1) {
        if *byte == b'+' || *byte == b'-' {
            let offset = parse_signed_decimal_wide(&input[index..]).unwrap_or(i64::MAX);
            return (&input[..index], offset);
        }
    }
    (input, 0)
}

fn find_symbol(input: &[u8]) -> Option<u64> {
    for index in 0..MAX_SYMBOLS {
        let symbol = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SYMBOLS[index])) };
        if symbol.enabled && &symbol.name[..symbol.length] == input {
            return Some(symbol.address);
        }
    }
    None
}

fn make_symbol(name: &[u8], address: u64) -> Option<GuestSymbol> {
    if name.is_empty() || name.len() >= SYMBOL_NAME_CAPACITY {
        return None;
    }
    if !name
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_' || *byte == b'.' || *byte == b'$')
    {
        return None;
    }
    let mut symbol = GuestSymbol::empty();
    symbol.name[..name.len()].copy_from_slice(name);
    symbol.length = name.len();
    symbol.address = address;
    symbol.enabled = true;
    Some(symbol)
}

fn step_target(context: *mut TargetContext) -> ! {
    let context = unsafe { &mut *context };
    if context.mcause != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        monitor_loop(context);
    }

    let current_pc = context.pc;
    let current_word = match target_load32(current_pc) {
        Some(word) => word,
        None => {
            uart_write("error: current pc is outside target RAM\r\n");
            monitor_loop(context);
        }
    };
    let instruction_pc = if current_word == EBREAK_WORD {
        if let Some(slot) = permanent_breakpoint_at(current_pc) {
            if !prepare_permanent_breakpoint_step(context, slot) {
                uart_write("error: cannot step over permanent breakpoint\r\n");
                monitor_loop(context);
            }
            unsafe { resume_user(context as *mut TargetContext) }
        }
        match current_pc.checked_add(4) {
            Some(address) => address,
            None => {
                uart_write("error: breakpoint pc overflow\r\n");
                monitor_loop(context);
            }
        }
    } else {
        current_pc
    };
    let instruction_word = match target_load32(instruction_pc) {
        Some(word) => word,
        None => {
            uart_write("error: instruction pc is outside target RAM\r\n");
            monitor_loop(context);
        }
    };
    if instruction_word == EBREAK_WORD {
        context.pc = instruction_pc;
        context.mepc = instruction_pc;
        unsafe { resume_user(context as *mut TargetContext) }
    }
    let stop_pc = match next_execution_pc(context, instruction_pc, instruction_word) {
        Some(address) => address,
        None => {
            uart_write("error: unsupported control-flow instruction for step\r\n");
            monitor_loop(context);
        }
    };
    if !install_temporary_breakpoint(stop_pc) {
        uart_write("error: cannot install temporary breakpoint\r\n");
        monitor_loop(context);
    }
    context.pc = instruction_pc;
    context.mepc = instruction_pc;
    unsafe { resume_user(context as *mut TargetContext) }
}

fn continue_target(context: *mut TargetContext) -> ! {
    let context = unsafe { &mut *context };
    if context.mcause != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        monitor_loop(context);
    }
    if target_load32(context.pc) == Some(EBREAK_WORD) {
        if let Some(slot) = permanent_breakpoint_at(context.pc) {
            if !prepare_permanent_breakpoint_step(context, slot) {
                uart_write("error: cannot continue over permanent breakpoint\r\n");
                monitor_loop(context);
            }
            unsafe { resume_user(context as *mut TargetContext) }
        }
    }
    let resume_pc = match target_load32(context.pc) {
        Some(EBREAK_WORD) => match context.pc.checked_add(4) {
            Some(address) => address,
            None => {
                uart_write("error: breakpoint pc overflow\r\n");
                monitor_loop(context);
            }
        },
        Some(_) => context.pc,
        None => {
            uart_write("error: current pc is outside target RAM\r\n");
            monitor_loop(context);
        }
    };
    context.pc = resume_pc;
    context.mepc = resume_pc;
    unsafe { resume_user(context as *mut TargetContext) }
}

fn break_target(argument: &[u8]) {
    let Some(address) = parse_address_or_symbol(argument) else {
        uart_write("error: break expects an address or a known label\r\n");
        return;
    };
    if !valid_target_word_address(address) {
        uart_write("error: breakpoint address must be an aligned target RAM word\r\n");
        return;
    }
    if let Some(slot) = permanent_breakpoint_at(address) {
        uart_write("breakpoint #");
        uart_decimal((slot + 1) as u64);
        uart_write(" already enabled\r\n");
        return;
    }
    if temporary_breakpoint_at(address) {
        uart_write("error: address is used by the temporary step breakpoint\r\n");
        return;
    }
    let Some(original_word) = target_load32(address) else {
        uart_write("error: cannot read breakpoint address\r\n");
        return;
    };
    let Some(slot) = first_free_permanent_breakpoint() else {
        uart_write("error: permanent breakpoint table is full\r\n");
        return;
    };
    clear_memory_undo();
    if !target_store32(address, EBREAK_WORD) {
        uart_write("error: cannot write breakpoint address\r\n");
        return;
    }
    flush_icache();
    let breakpoint = Breakpoint {
        address,
        original_word,
        enabled: true,
    };
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(PERMANENT_BREAKPOINTS[slot]),
            breakpoint,
        );
    }
    uart_write("breakpoint #");
    uart_decimal((slot + 1) as u64);
    uart_write(" set at 0x");
    uart_hex(address);
    uart_write("\r\n");
}

fn delete_breakpoint(argument: &[u8]) {
    let Some(number) = parse_decimal(argument) else {
        uart_write("error: delete expects a breakpoint number\r\n");
        return;
    };
    if number == 0 || number > MAX_PERMANENT_BREAKPOINTS as u64 {
        uart_write("error: breakpoint number is out of range\r\n");
        return;
    }
    let slot = (number - 1) as usize;
    let breakpoint =
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PERMANENT_BREAKPOINTS[slot])) };
    if !breakpoint.enabled {
        uart_write("error: breakpoint is not enabled\r\n");
        return;
    }
    if target_load32(breakpoint.address) != Some(EBREAK_WORD) {
        uart_write("error: breakpoint memory was modified; refusing to restore it\r\n");
        return;
    }
    clear_memory_undo();
    if !target_store32(breakpoint.address, breakpoint.original_word) {
        uart_write("error: cannot restore breakpoint memory\r\n");
        return;
    }
    flush_icache();
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(PERMANENT_BREAKPOINTS[slot]),
            Breakpoint::disabled(),
        );
    }
    uart_write("breakpoint #");
    uart_decimal(number);
    uart_write(" deleted\r\n");
}

fn print_breakpoints() {
    uart_write("breakpoints:\r\n");
    let mut found = false;
    let mut slot = 0;
    while slot < MAX_PERMANENT_BREAKPOINTS {
        let breakpoint =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PERMANENT_BREAKPOINTS[slot])) };
        if breakpoint.enabled {
            found = true;
            uart_write("  #");
            uart_decimal((slot + 1) as u64);
            uart_write(" addr=0x");
            uart_hex(breakpoint.address);
            uart_write(" original=0x");
            uart_hex(u64::from(breakpoint.original_word));
            uart_write("\r\n");
        }
        slot += 1;
    }
    if !found {
        uart_write("  none\r\n");
    }
}

// This is deliberately only the control-flow successor calculation needed by
// the temporary-breakpoint stepper. The instruction registry remains owned by
// the generated host-side ISA tables; this is not a second opcode table.
fn next_execution_pc(context: &TargetContext, pc: u64, word: u32) -> Option<u64> {
    match word & 0x7f {
        0x63 => {
            let rs1 = ((word >> 15) & 0x1f) as usize;
            let rs2 = ((word >> 20) & 0x1f) as usize;
            let immediate = (((word >> 31) & 1) << 12)
                | (((word >> 25) & 0x3f) << 5)
                | (((word >> 8) & 0xf) << 1)
                | (((word >> 7) & 1) << 11);
            let immediate = ((immediate as i32) << 19 >> 19) as i64 as u64;
            let taken = match (word >> 12) & 0x7 {
                0b000 => context.x[rs1] == context.x[rs2],
                0b001 => context.x[rs1] != context.x[rs2],
                _ => return None,
            };
            Some(if taken {
                pc.wrapping_add(immediate)
            } else {
                pc.wrapping_add(4)
            })
        }
        0x6f => {
            let immediate = (((word >> 31) & 1) << 20)
                | (((word >> 21) & 0x3ff) << 1)
                | (((word >> 20) & 1) << 11)
                | (((word >> 12) & 0xff) << 12);
            let immediate = ((immediate as i32) << 11 >> 11) as i64 as u64;
            Some(pc.wrapping_add(immediate))
        }
        0x67 if (word >> 12) & 0x7 == 0 => {
            let rs1 = ((word >> 15) & 0x1f) as usize;
            let immediate = (word as i32 >> 20) as i64 as u64;
            Some(context.x[rs1].wrapping_add(immediate) & !1)
        }
        0x67 => None,
        _ => pc.checked_add(4),
    }
}

fn install_temporary_breakpoint(address: u64) -> bool {
    if permanent_breakpoint_at(address).is_some() {
        return false;
    }
    let Some(original_word) = target_load32(address) else {
        return false;
    };
    clear_memory_undo();
    if !target_store32(address, EBREAK_WORD) {
        return false;
    }
    flush_icache();
    let breakpoint = Breakpoint {
        address,
        original_word,
        enabled: true,
    };
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(TEMPORARY_BREAKPOINT), breakpoint);
    }
    true
}

fn restore_temporary_breakpoint() {
    let breakpoint = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(TEMPORARY_BREAKPOINT)) };
    if breakpoint.enabled {
        if target_store32(breakpoint.address, breakpoint.original_word) {
            flush_icache();
        }
        unsafe {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(TEMPORARY_BREAKPOINT),
                Breakpoint::disabled(),
            );
        }
        uart_write("step: temporary breakpoint restored\r\n");
    }
}

fn prepare_permanent_breakpoint_step(context: &mut TargetContext, slot: usize) -> bool {
    let breakpoint =
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PERMANENT_BREAKPOINTS[slot])) };
    if !breakpoint.enabled || breakpoint.original_word == EBREAK_WORD {
        return false;
    }
    let Some(stop_pc) = next_execution_pc(context, context.pc, breakpoint.original_word) else {
        return false;
    };
    if stop_pc == context.pc || !valid_target_word_address(stop_pc) {
        return false;
    }
    if !target_store32(breakpoint.address, breakpoint.original_word) {
        return false;
    }
    flush_icache();
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(STEPPED_PERMANENT_BREAKPOINT),
            slot as u8,
        );
    }
    if permanent_breakpoint_at(stop_pc).is_none() && !install_temporary_breakpoint(stop_pc) {
        restore_stepped_permanent_breakpoint();
        return false;
    }
    context.mepc = context.pc;
    true
}

fn restore_stepped_permanent_breakpoint() {
    let slot =
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!(STEPPED_PERMANENT_BREAKPOINT)) };
    if slot == u8::MAX {
        return;
    }
    let slot = usize::from(slot);
    let breakpoint =
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PERMANENT_BREAKPOINTS[slot])) };
    if breakpoint.enabled && target_load32(breakpoint.address) == Some(breakpoint.original_word) {
        if target_store32(breakpoint.address, EBREAK_WORD) {
            flush_icache();
        }
    }
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(STEPPED_PERMANENT_BREAKPOINT),
            u8::MAX,
        );
    }
}

fn target_load32(address: u64) -> Option<u32> {
    if !valid_target_word_address(address) {
        return None;
    }
    Some(unsafe { core::ptr::read_volatile(address as *const u32) })
}

fn target_store32(address: u64, word: u32) -> bool {
    if !valid_target_word_address(address) {
        return false;
    }
    unsafe { core::ptr::write_volatile(address as *mut u32, word) };
    true
}

fn valid_target_word_address(address: u64) -> bool {
    address % 4 == 0
        && address >= TARGET_RAM_START
        && address
            .checked_add(4)
            .is_some_and(|end| end <= TARGET_RAM_END)
}

fn valid_target_program_word_address(address: u64) -> bool {
    valid_target_word_address(address)
        && address >= target_workspace_start()
        && address
            .checked_add(4)
            .is_some_and(|end| end <= target_workspace_end())
}

fn target_workspace_start() -> u64 {
    core::ptr::addr_of!(_target_workspace_start) as u64
}

fn target_workspace_end() -> u64 {
    core::ptr::addr_of!(_target_workspace_end) as u64
}

fn permanent_breakpoint_at(address: u64) -> Option<usize> {
    let mut slot = 0;
    while slot < MAX_PERMANENT_BREAKPOINTS {
        let breakpoint =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PERMANENT_BREAKPOINTS[slot])) };
        if breakpoint.enabled && breakpoint.address == address {
            return Some(slot);
        }
        slot += 1;
    }
    None
}

fn temporary_breakpoint_at(address: u64) -> bool {
    let breakpoint = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(TEMPORARY_BREAKPOINT)) };
    breakpoint.enabled && breakpoint.address == address
}

fn first_free_permanent_breakpoint() -> Option<usize> {
    let mut slot = 0;
    while slot < MAX_PERMANENT_BREAKPOINTS {
        let breakpoint =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PERMANENT_BREAKPOINTS[slot])) };
        if !breakpoint.enabled {
            return Some(slot);
        }
        slot += 1;
    }
    None
}

fn flush_icache() {
    unsafe {
        asm!("fence.i", options(nostack));
    }
}

fn uart_read_line(buffer: &mut [u8; COMMAND_CAPACITY]) -> usize {
    let mut length = 0;
    loop {
        let byte = uart_get();
        match byte {
            b'\r' | b'\n' => {
                uart_write("\r\n");
                return length;
            }
            8 | 127 if length > 0 => {
                length -= 1;
                uart_write("\x08 \x08");
            }
            32..=126 if length < buffer.len() => {
                buffer[length] = byte.to_ascii_lowercase();
                length += 1;
                uart_put(byte);
            }
            _ => {}
        }
    }
}

fn uart_hex(value: u64) {
    for shift in (0..16).rev() {
        let digit = ((value >> (shift * 4)) & 0xf) as u8;
        uart_put(if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        });
    }
}

fn uart_hex_byte(value: u8) {
    let high = value >> 4;
    let low = value & 0xf;
    uart_put(hex_digit(high));
    uart_put(hex_digit(low));
}

fn hex_digit(value: u8) -> u8 {
    if value < 10 {
        b'0' + value
    } else {
        b'a' + value - 10
    }
}

fn uart_decimal(mut value: u64) {
    let mut digits = [0u8; 20];
    let mut length = 0;
    if value == 0 {
        uart_put(b'0');
        return;
    }
    while value != 0 {
        digits[length] = b'0' + (value % 10) as u8;
        length += 1;
        value /= 10;
    }
    while length != 0 {
        length -= 1;
        uart_put(digits[length]);
    }
}

fn uart_signed_decimal(value: i16) {
    if value < 0 {
        uart_put(b'-');
        uart_decimal(u64::from(value.unsigned_abs()));
    } else {
        uart_decimal(u64::from(value as u16));
    }
}

fn parse_hex(input: &[u8]) -> Option<u64> {
    let input = input.strip_prefix(b"0x").unwrap_or(input);
    if input.is_empty() {
        return None;
    }
    let mut value = 0u64;
    for &byte in input {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => return None,
        };
        value = value.checked_mul(16)?.checked_add(u64::from(digit))?;
    }
    Some(value)
}

fn parse_hex_bytes(input: &[u8], destination: &mut [u8; MAX_EDIT_BYTES]) -> Option<usize> {
    let input = input.strip_prefix(b"0x").unwrap_or(input);
    if input.is_empty() || input.len() % 2 != 0 || input.len() / 2 > destination.len() {
        return None;
    }
    for (index, pair) in input.chunks_exact(2).enumerate() {
        destination[index] = (parse_hex_nibble(pair[0])? << 4) | parse_hex_nibble(pair[1])?;
    }
    Some(input.len() / 2)
}

fn parse_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn parse_decimal(input: &[u8]) -> Option<u64> {
    if input.is_empty() {
        return None;
    }
    let mut value = 0u64;
    for &byte in input {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u64::from(byte - b'0'))?;
    }
    Some(value)
}

unsafe extern "C" {
    fn install_trap(mtvec: usize, context: usize);
    fn enter_user(pc: usize, stack: usize) -> !;
    fn resume_user(context: *mut TargetContext) -> !;
    fn target_entry() -> !;
    fn trap_entry();
    static _target_workspace_start: u8;
    static _target_workspace_end: u8;
}

fn uart_write(text: &str) {
    for byte in text.bytes() {
        uart_put(byte);
    }
}

fn uart_bytes(bytes: &[u8]) {
    for byte in bytes {
        uart_put(*byte);
    }
}

fn uart_put(byte: u8) {
    while unsafe { core::ptr::read_volatile((UART_BASE + UART_LSR) as *const u8) } & UART_LSR_EMPTY
        == 0
    {}
    unsafe {
        core::ptr::write_volatile(UART_BASE as *mut u8, byte);
    }
}

fn uart_get() -> u8 {
    while unsafe { core::ptr::read_volatile((UART_BASE + UART_LSR) as *const u8) }
        & UART_LSR_DATA_READY
        == 0
    {}
    unsafe { core::ptr::read_volatile(UART_BASE as *const u8) }
}
