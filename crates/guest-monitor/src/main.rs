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
const UART_FCR: usize = 2;
const UART_LSR: usize = 5;
const UART_FCR_ENABLE_FIFO: u8 = 1 << 0;
const UART_FCR_TRIGGER_1: u8 = 0;
const UART_LSR_DATA_READY: u8 = 1 << 0;
const UART_LSR_EMPTY: u8 = 1 << 5;
const UART_RX_BUFFER_CAPACITY: usize = 64;
const COMMAND_CAPACITY: usize = 96;
const TARGET_RAM_START: u64 = 0x8000_0000;
const TARGET_RAM_END: u64 = 0x8400_0000;
const TARGET_WORKSPACE_BYTES: usize = 0x1_0000;
const TARGET_DATA_BYTES: usize = 0x10_0000;
const TARGET_WORKSPACE_START: u64 = 0x8100_0000;
const TARGET_DATA_START: u64 = 0x8200_0000;
const EBREAK_WORD: u32 = 0x0010_0073;
const MAX_PERMANENT_BREAKPOINTS: usize = 4;
const MAX_WATCHPOINTS: usize = 4;
const MAX_MEMORY_DUMP: u64 = 128;
const MAX_EDIT_BYTES: usize = 32;
const MAX_SNAPSHOT_DUMP: u64 = 4096;
const MAX_METADATA_BYTES: usize = 4096;
const MAX_SOURCE_LINES: usize = 16;
const MAX_SYMBOLS: usize = 8;
const SYMBOL_NAME_CAPACITY: usize = 16;

global_asm!(include_str!("entry.S"));

static mut CONTEXT: TargetContext = TargetContext::empty();
static mut TEMPORARY_BREAKPOINT: Breakpoint = Breakpoint::disabled();
static mut PERMANENT_BREAKPOINTS: [Breakpoint; MAX_PERMANENT_BREAKPOINTS] =
    [Breakpoint::disabled(); MAX_PERMANENT_BREAKPOINTS];
static mut WATCHPOINTS: [GuestWatchpoint; MAX_WATCHPOINTS] =
    [GuestWatchpoint::disabled(); MAX_WATCHPOINTS];
static mut STEPPED_PERMANENT_BREAKPOINT: u8 = u8::MAX;
static mut RUN_REMAINING: u64 = 0;
static mut SYMBOLS: [GuestSymbol; MAX_SYMBOLS] = [GuestSymbol::empty(); MAX_SYMBOLS];
static mut SOURCE_LINES: [[u8; COMMAND_CAPACITY]; MAX_SOURCE_LINES] =
    [[0; COMMAND_CAPACITY]; MAX_SOURCE_LINES];
static mut SOURCE_LENGTHS: [usize; MAX_SOURCE_LINES] = [0; MAX_SOURCE_LINES];
static mut SOURCE_COUNT: usize = 0;
static mut SOURCE_ADDRESS: u64 = 0;
static mut MEMORY_UNDO: MemoryUndo = MemoryUndo::empty();
static mut GUEST_SNAPSHOT: GuestSnapshot = GuestSnapshot::empty();
static mut SNAPSHOT_BINARY_PATCH: [u8; MAX_SNAPSHOT_DUMP as usize] =
    [0; MAX_SNAPSHOT_DUMP as usize];
static mut SNAPSHOT_BINARY_COMPRESSED: [u8; MAX_SNAPSHOT_DUMP as usize] =
    [0; MAX_SNAPSHOT_DUMP as usize];
static mut SNAPSHOT_METADATA: [u8; MAX_METADATA_BYTES] = [0; MAX_METADATA_BYTES];
static mut UART_RX_BUFFER: [u8; UART_RX_BUFFER_CAPACITY] = [0; UART_RX_BUFFER_CAPACITY];
static mut UART_RX_LENGTH: usize = 0;
static mut UART_RX_INDEX: usize = 0;
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

#[derive(Clone, Copy)]
struct GuestSnapshot {
    valid: bool,
    context: TargetContext,
    workspace: [u8; TARGET_WORKSPACE_BYTES],
    data: [u8; TARGET_DATA_BYTES],
    source_lines: [[u8; COMMAND_CAPACITY]; MAX_SOURCE_LINES],
    source_lengths: [usize; MAX_SOURCE_LINES],
    source_count: usize,
    source_address: u64,
    symbols: [GuestSymbol; MAX_SYMBOLS],
    breakpoints: [Breakpoint; MAX_PERMANENT_BREAKPOINTS],
    watchpoints: [GuestWatchpoint; MAX_WATCHPOINTS],
}

impl GuestSnapshot {
    const fn empty() -> Self {
        Self {
            valid: false,
            context: TargetContext::empty(),
            workspace: [0; TARGET_WORKSPACE_BYTES],
            data: [0; TARGET_DATA_BYTES],
            source_lines: [[0; COMMAND_CAPACITY]; MAX_SOURCE_LINES],
            source_lengths: [0; MAX_SOURCE_LINES],
            source_count: 0,
            source_address: 0,
            symbols: [GuestSymbol::empty(); MAX_SYMBOLS],
            breakpoints: [Breakpoint::disabled(); MAX_PERMANENT_BREAKPOINTS],
            watchpoints: [GuestWatchpoint::disabled(); MAX_WATCHPOINTS],
        }
    }
}

#[derive(Clone, Copy)]
enum GuestWatchKind {
    Read,
    Write,
    Any,
}

#[derive(Clone, Copy)]
struct GuestWatchpoint {
    address: u64,
    width: u64,
    kind: GuestWatchKind,
    enabled: bool,
}

impl GuestWatchpoint {
    const fn disabled() -> Self {
        Self {
            address: 0,
            width: 0,
            kind: GuestWatchKind::Any,
            enabled: false,
        }
    }
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
    uart_init();
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
    uart_write("target data: 0x");
    uart_hex(target_data_start());
    uart_write("..0x");
    uart_hex(target_data_end());
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
    let budget_stop = context.mcause == StopReason::Breakpoint as u64
        && permanent_breakpoint_at(context.mepc).is_none()
        && target_load32(context.mepc) != Some(EBREAK_WORD);
    if budget_stop {
        let remaining = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(RUN_REMAINING)) };
        if remaining > 0 {
            unsafe {
                core::ptr::write_volatile(core::ptr::addr_of_mut!(RUN_REMAINING), remaining - 1);
            }
            if remaining > 1 {
                resume_single_step(context as *mut TargetContext);
            }
        }
    } else {
        unsafe {
            core::ptr::write_volatile(core::ptr::addr_of_mut!(RUN_REMAINING), 0);
        }
    }
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
    if budget_stop {
        uart_write("run: budget exhausted\r\n");
    }
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
            command if command.starts_with(b"set ") => set_integer_register(context, &command[4..]),
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
            b"source" => print_source(None),
            command if command.starts_with(b"source ") => source_command(&command[7..]),
            b"assemble-source" => assemble_saved_source(context),
            b"snapshot save" | b"project-save" => save_guest_snapshot(context),
            b"snapshot restore" | b"project-load" => restore_guest_snapshot(context),
            b"snapshot info" => snapshot_info(),
            b"snapshot manifest" => snapshot_manifest(),
            b"snapshot metadata" => snapshot_metadata_info(),
            command if command.starts_with(b"snapshot metadata dump ") => {
                snapshot_metadata_dump(&command[23..])
            }
            command if command.starts_with(b"snapshot dump ") => snapshot_dump(&command[14..]),
            command if command.starts_with(b"snapshot patchbin ") => {
                snapshot_patch_binary(&command[18..])
            }
            command if command.starts_with(b"snapshot patchrle ") => {
                snapshot_patch_rle(&command[18..])
            }
            command if command.starts_with(b"snapshot patch ") => snapshot_patch(&command[15..]),
            b"symbols" => print_symbols(),
            command if command.starts_with(b"disasm ") => print_disassembly(&command[7..]),
            b"step" | b"s" => step_target(context),
            command if command.starts_with(b"run ") => run_target(context, &command[4..]),
            b"continue" | b"c" => continue_target(context),
            command if command.starts_with(b"break ") => break_target(&command[6..]),
            command if command.starts_with(b"watch ") => {
                add_watchpoint(&command[6..], GuestWatchKind::Write)
            }
            command if command.starts_with(b"rwatch ") => {
                add_watchpoint(&command[7..], GuestWatchKind::Read)
            }
            command if command.starts_with(b"awatch ") => {
                add_watchpoint(&command[7..], GuestWatchKind::Any)
            }
            b"info watch" | b"info w" => print_watchpoints(),
            command if command.starts_with(b"delete watch ") => delete_watchpoint(&command[13..]),
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
        "help/? regs/registers set <xreg> <hex64> setf <freg> <hex64> memory <addr> <length> edit <addr> <hex-bytes> data <addr> <directive> <bits> undo assemble <addr> <instruction> assemble-program <addr> ... end assemble-source source [line]|replace <n> <text> snapshot save|restore|info|manifest|metadata|dump|patch|patchbin|patchrle project-save|project-load symbols disasm <addr|label> <count> step/s run <count> continue/c break <addr|label> watch/rwatch/awatch <addr> <width> delete <n>|watch <n> info break/watch quit/q\r\n",
    );
}

fn guest_error(code: &[u8], message: &[u8]) {
    uart_write("error [");
    uart_bytes(code);
    uart_write("]: ");
    uart_bytes(message);
    uart_write("\r\n");
}

fn guest_source_error(line: usize, code: &[u8], message: &[u8]) {
    uart_write("error [");
    uart_bytes(code);
    uart_write("] source line ");
    uart_decimal(line as u64);
    uart_write(": ");
    uart_bytes(message);
    uart_write("\r\n");
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

fn set_integer_register(context: *mut TargetContext, argument: &[u8]) {
    if unsafe { (*context).mcause } != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        return;
    }
    let Some((register_bytes, value_bytes)) = split_token_space(argument) else {
        uart_write("error: set expects <xreg> <hex64>\r\n");
        return;
    };
    let Some(register) = parse_register(register_bytes) else {
        uart_write("error: set register must be x0..x31\r\n");
        return;
    };
    if register == 0 {
        uart_write("error: x0 is read-only\r\n");
        return;
    }
    let Some(value) = parse_hex(value_bytes) else {
        uart_write("error: set value must be a hexadecimal 64-bit pattern\r\n");
        return;
    };
    unsafe {
        (*context).x[usize::from(register)] = value;
    }
    uart_write("set x");
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
    if !valid_target_data_range(address, length) {
        uart_write("error: data range is outside target data region\r\n");
        return;
    }
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
    for (mnemonic, rd_float, rs1_float) in [
        ("fmv.w.x", true, false),
        ("fmv.x.w", false, true),
        ("fmv.d.x", true, false),
        ("fmv.x.d", false, true),
    ] {
        let Some(opcode) = GENERATED_OPCODES
            .iter()
            .find(|opcode| opcode.mnemonic == mnemonic)
        else {
            continue;
        };
        if word & opcode.mask != opcode.match_value {
            continue;
        }
        uart_write(mnemonic);
        uart_write(" ");
        if rd_float {
            uart_write("f");
        } else {
            uart_write("x");
        }
        uart_decimal(u64::from((word >> 7) & 31));
        uart_write(",");
        if rs1_float {
            uart_write("f");
        } else {
            uart_write("x");
        }
        uart_decimal(u64::from((word >> 15) & 31));
        return;
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
        "source mode: enter integer/control, ld/sd, fadd.s/fadd.d or fmv lines, finish with end\r\n",
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
        guest_error(
            b"GUEST-ASM-001",
            b"source program exceeds 16 instruction lines",
        );
        return;
    }
    if count == 0 {
        guest_error(b"GUEST-ASM-002", b"source program is empty");
        return;
    }
    let mut staged_symbols = [GuestSymbol::empty(); MAX_SYMBOLS];
    let mut instruction_count = 0usize;
    for index in 0..count {
        let line = &lines[index][..lengths[index]];
        if let Some(label) = line.strip_suffix(b":") {
            let line_address = address + (instruction_count as u64) * 4;
            let Some(symbol) = make_symbol(label, line_address) else {
                guest_source_error(
                    index + 1,
                    b"GUEST-ASM-003",
                    b"invalid, duplicate or too many source labels",
                );
                return;
            };
            if staged_symbols
                .iter()
                .any(|slot| slot.enabled && &slot.name[..slot.length] == label)
            {
                guest_source_error(
                    index + 1,
                    b"GUEST-ASM-003",
                    b"invalid, duplicate or too many source labels",
                );
                return;
            }
            if let Some(slot) = staged_symbols.iter_mut().find(|slot| !slot.enabled) {
                *slot = symbol;
            } else {
                guest_source_error(
                    index + 1,
                    b"GUEST-ASM-003",
                    b"invalid, duplicate or too many source labels",
                );
                return;
            }
        } else {
            instruction_count += 1;
        }
    }
    if instruction_count == 0 {
        guest_error(b"GUEST-ASM-004", b"source program contains no instructions");
        return;
    }

    let Some(end_address) = address.checked_add((instruction_count as u64) * 4) else {
        guest_error(b"GUEST-ASM-005", b"source program address overflows");
        return;
    };
    if end_address > TARGET_RAM_END {
        guest_error(
            b"GUEST-ASM-006",
            b"source program does not fit in target RAM",
        );
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
            guest_source_error(
                index + 1,
                b"GUEST-ASM-007",
                b"source program exceeds target workspace",
            );
            return;
        }
        let Some(word) = parse_source_instruction(line, line_address, &staged_symbols) else {
            guest_source_error(
                index + 1,
                b"GUEST-ASM-008",
                b"supports integer/control, ld/sd, fadd.s/fadd.d or fmv syntax",
            );
            return;
        };
        if permanent_breakpoint_at(line_address).is_some() || temporary_breakpoint_at(line_address)
        {
            guest_source_error(
                index + 1,
                b"GUEST-ASM-009",
                b"source overlaps an active breakpoint",
            );
            return;
        }
        words[word_count] = word;
        word_addresses[word_count] = line_address;
        word_count += 1;
    }

    clear_memory_undo();
    for index in 0..word_count {
        if !target_store32(word_addresses[index], words[index]) {
            guest_error(b"GUEST-ASM-010", b"cannot write assembled source program");
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
    store_source(&lines, &lengths, count, address);
    uart_write("assembled program: ");
    uart_decimal(word_count as u64);
    uart_write(" instruction(s) at 0x");
    uart_hex(address);
    uart_write("\r\n");
}

fn store_source(
    lines: &[[u8; COMMAND_CAPACITY]; MAX_SOURCE_LINES],
    lengths: &[usize; MAX_SOURCE_LINES],
    count: usize,
    address: u64,
) {
    unsafe {
        for index in 0..MAX_SOURCE_LINES {
            core::ptr::write_volatile(core::ptr::addr_of_mut!(SOURCE_LINES[index]), lines[index]);
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(SOURCE_LENGTHS[index]),
                lengths[index],
            );
        }
        core::ptr::write_volatile(core::ptr::addr_of_mut!(SOURCE_COUNT), count);
        core::ptr::write_volatile(core::ptr::addr_of_mut!(SOURCE_ADDRESS), address);
    }
}

fn print_source(line: Option<usize>) {
    let count = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_COUNT)) };
    if count == 0 {
        uart_write("source: empty\r\n");
        return;
    }
    if let Some(line) = line {
        if line == 0 || line > count {
            guest_error(
                b"GUEST-SOURCE-001",
                b"source line is outside the loaded document",
            );
            return;
        }
        uart_decimal(line as u64);
        uart_write(" | ");
        let length =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_LENGTHS[line - 1])) };
        let bytes = unsafe { &*core::ptr::addr_of!(SOURCE_LINES[line - 1]) };
        uart_bytes(&bytes[..length]);
        uart_write("\r\n");
        return;
    }
    for index in 0..count {
        uart_decimal((index + 1) as u64);
        uart_write(" | ");
        let length =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_LENGTHS[index])) };
        let bytes = unsafe { &*core::ptr::addr_of!(SOURCE_LINES[index]) };
        uart_bytes(&bytes[..length]);
        uart_write("\r\n");
    }
}

fn source_command(argument: &[u8]) {
    if let Some(spec) = argument.strip_prefix(b"replace ") {
        let Some((line_bytes, replacement)) = split_token_space(spec) else {
            guest_error(b"GUEST-SOURCE-002", b"source replace expects <line> <text>");
            return;
        };
        let Some(line) = parse_decimal(line_bytes) else {
            guest_error(b"GUEST-SOURCE-003", b"source line must be decimal");
            return;
        };
        let count = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_COUNT)) };
        if line == 0 || line > count as u64 {
            guest_error(
                b"GUEST-SOURCE-001",
                b"source line is outside the loaded document",
            );
            return;
        }
        let replacement = replacement
            .strip_prefix(b"\"")
            .and_then(|value| value.strip_suffix(b"\""))
            .unwrap_or(replacement);
        if replacement.len() > COMMAND_CAPACITY {
            guest_error(b"GUEST-SOURCE-004", b"replacement line is too long");
            return;
        }
        let index = line as usize - 1;
        let mut updated = [0u8; COMMAND_CAPACITY];
        updated[..replacement.len()].copy_from_slice(replacement);
        unsafe {
            core::ptr::write_volatile(core::ptr::addr_of_mut!(SOURCE_LINES[index]), updated);
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(SOURCE_LENGTHS[index]),
                replacement.len(),
            );
        }
        uart_write("source line ");
        uart_decimal(line);
        uart_write(" updated; use assemble-source to apply\r\n");
        return;
    }
    let Some(line) = parse_decimal(argument.trim_ascii()) else {
        guest_error(
            b"GUEST-SOURCE-005",
            b"source expects a line or replace command",
        );
        return;
    };
    print_source(Some(line as usize));
}

fn assemble_saved_source(context: *mut TargetContext) {
    let count = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_COUNT)) };
    if count == 0 {
        guest_error(b"GUEST-SOURCE-006", b"no source document is loaded");
        monitor_loop(context);
    }
    let address = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_ADDRESS)) };
    let mut lines = [[0u8; COMMAND_CAPACITY]; MAX_SOURCE_LINES];
    let mut lengths = [0usize; MAX_SOURCE_LINES];
    unsafe {
        for index in 0..count {
            lines[index] = core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_LINES[index]));
            lengths[index] = core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_LENGTHS[index]));
        }
    }
    assemble_source_buffer(context, address, &lines, &lengths, count);
}

fn snapshot_region(address: u64, length: usize) -> Option<(bool, usize)> {
    let end = address.checked_add(length as u64)?;
    let workspace_end = TARGET_WORKSPACE_START + TARGET_WORKSPACE_BYTES as u64;
    if address >= TARGET_WORKSPACE_START && end <= workspace_end {
        return Some((true, (address - TARGET_WORKSPACE_START) as usize));
    }
    let data_end = TARGET_DATA_START + TARGET_DATA_BYTES as u64;
    if address >= TARGET_DATA_START && end <= data_end {
        return Some((false, (address - TARGET_DATA_START) as usize));
    }
    None
}

fn save_guest_snapshot(context: *mut TargetContext) {
    let context_value = unsafe { core::ptr::read_volatile(context) };
    if context_value.mcause != StopReason::Breakpoint as u64 {
        guest_error(
            b"GUEST-SNAPSHOT-001",
            b"target must be stopped at a breakpoint",
        );
        return;
    }
    let snapshot = core::ptr::addr_of_mut!(GUEST_SNAPSHOT);
    unsafe {
        (*snapshot).valid = false;
        (*snapshot).context = context_value;
        core::ptr::copy_nonoverlapping(
            TARGET_WORKSPACE_START as *const u8,
            (*snapshot).workspace.as_mut_ptr(),
            TARGET_WORKSPACE_BYTES,
        );
        core::ptr::copy_nonoverlapping(
            TARGET_DATA_START as *const u8,
            (*snapshot).data.as_mut_ptr(),
            TARGET_DATA_BYTES,
        );
        (*snapshot).source_count = core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_COUNT));
        (*snapshot).source_address = core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_ADDRESS));
        for index in 0..MAX_SOURCE_LINES {
            (*snapshot).source_lines[index] =
                core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_LINES[index]));
            (*snapshot).source_lengths[index] =
                core::ptr::read_volatile(core::ptr::addr_of!(SOURCE_LENGTHS[index]));
        }
        for index in 0..MAX_SYMBOLS {
            (*snapshot).symbols[index] =
                core::ptr::read_volatile(core::ptr::addr_of!(SYMBOLS[index]));
        }
        for index in 0..MAX_WATCHPOINTS {
            (*snapshot).watchpoints[index] =
                core::ptr::read_volatile(core::ptr::addr_of!(WATCHPOINTS[index]));
        }
        for index in 0..MAX_PERMANENT_BREAKPOINTS {
            let breakpoint =
                core::ptr::read_volatile(core::ptr::addr_of!(PERMANENT_BREAKPOINTS[index]));
            (*snapshot).breakpoints[index] = if breakpoint.enabled
                && snapshot_region(breakpoint.address, 4).is_some()
            {
                if let Some((workspace, offset)) = snapshot_region(breakpoint.address, 4) {
                    let bytes = breakpoint.original_word.to_le_bytes();
                    if workspace {
                        (&mut (*snapshot).workspace)[offset..offset + 4].copy_from_slice(&bytes);
                    } else {
                        (&mut (*snapshot).data)[offset..offset + 4].copy_from_slice(&bytes);
                    }
                }
                breakpoint
            } else {
                Breakpoint::disabled()
            };
        }
        (*snapshot).valid = true;
    }
    uart_write("snapshot saved (workspace=65536 data=1048576)\r\n");
}

fn restore_guest_snapshot(context: *mut TargetContext) {
    let snapshot = core::ptr::addr_of_mut!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        guest_error(b"GUEST-SNAPSHOT-002", b"no snapshot or project is saved");
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(
            (*snapshot).workspace.as_ptr(),
            TARGET_WORKSPACE_START as *mut u8,
            TARGET_WORKSPACE_BYTES,
        );
        core::ptr::copy_nonoverlapping(
            (*snapshot).data.as_ptr(),
            TARGET_DATA_START as *mut u8,
            TARGET_DATA_BYTES,
        );
        for index in 0..MAX_PERMANENT_BREAKPOINTS {
            let breakpoint = (*snapshot).breakpoints[index];
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(PERMANENT_BREAKPOINTS[index]),
                breakpoint,
            );
            if breakpoint.enabled {
                let _ = target_store32(breakpoint.address, EBREAK_WORD);
            }
        }
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(TEMPORARY_BREAKPOINT),
            Breakpoint::disabled(),
        );
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(STEPPED_PERMANENT_BREAKPOINT),
            u8::MAX,
        );
        for index in 0..MAX_WATCHPOINTS {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(WATCHPOINTS[index]),
                (*snapshot).watchpoints[index],
            );
        }
        for index in 0..MAX_SOURCE_LINES {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(SOURCE_LINES[index]),
                (*snapshot).source_lines[index],
            );
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(SOURCE_LENGTHS[index]),
                (*snapshot).source_lengths[index],
            );
        }
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(SOURCE_COUNT),
            (*snapshot).source_count,
        );
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(SOURCE_ADDRESS),
            (*snapshot).source_address,
        );
        for index in 0..MAX_SYMBOLS {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(SYMBOLS[index]),
                (*snapshot).symbols[index],
            );
        }
        core::ptr::write_volatile(core::ptr::addr_of_mut!(RUN_REMAINING), 0);
        core::ptr::write_volatile(core::ptr::addr_of_mut!(MEMORY_UNDO), MemoryUndo::empty());
        core::ptr::write_volatile(context, (*snapshot).context);
    }
    flush_icache();
    uart_write("snapshot restored (workspace=65536 data=1048576)\r\n");
}

fn snapshot_info() {
    let snapshot = core::ptr::addr_of!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        uart_write("snapshot: empty\r\n");
        return;
    }
    let source_count =
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).source_count)) };
    uart_write("snapshot: valid workspace=65536 data=1048576 source-lines=");
    uart_decimal(source_count as u64);
    uart_write(" chunk-max=4096\r\n");
}

fn snapshot_metadata_info() {
    let Some(length) = build_snapshot_metadata() else {
        guest_error(
            b"GUEST-SNAPSHOT-014",
            b"metadata does not fit its bounded buffer",
        );
        return;
    };
    uart_write("snapshot-metadata format=RVMETA01 size=");
    uart_decimal(length as u64);
    uart_write(" chunk-max=128\r\n");
}

fn snapshot_metadata_dump(argument: &[u8]) {
    let Some((offset_bytes, length_bytes)) = split_token_space(argument) else {
        guest_error(
            b"GUEST-SNAPSHOT-015",
            b"metadata dump expects <offset> <length>",
        );
        return;
    };
    let Some(offset) = parse_decimal(offset_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-005", b"snapshot offset must be decimal");
        return;
    };
    let Some(length) = parse_decimal(length_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-006", b"snapshot length must be decimal");
        return;
    };
    let Some(metadata_length) = build_snapshot_metadata() else {
        guest_error(
            b"GUEST-SNAPSHOT-014",
            b"metadata does not fit its bounded buffer",
        );
        return;
    };
    if length == 0
        || length > MAX_MEMORY_DUMP
        || offset
            .checked_add(length)
            .is_none_or(|end| end > metadata_length as u64)
    {
        guest_error(
            b"GUEST-SNAPSHOT-016",
            b"metadata chunk is outside its bounds",
        );
        return;
    }
    uart_write("snapshot-metadata-chunk offset=");
    uart_decimal(offset);
    uart_write(" length=");
    uart_decimal(length);
    uart_write(" hex=");
    unsafe {
        for byte in &SNAPSHOT_METADATA[offset as usize..(offset + length) as usize] {
            uart_hex_byte(*byte);
        }
    }
    uart_write("\r\n");
}

fn build_snapshot_metadata() -> Option<usize> {
    let snapshot = core::ptr::addr_of!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        return None;
    }
    let mut writer = MetadataWriter { offset: 0 };
    writer.bytes(b"RVMETA01")?;
    writer.u32(1)?;
    unsafe {
        for value in (*snapshot)
            .context
            .x
            .iter()
            .chain((*snapshot).context.f.iter())
        {
            writer.u64(*value)?;
        }
        writer.u64((*snapshot).context.pc)?;
        writer.u32((*snapshot).context.fcsr)?;
        writer.u64((*snapshot).context.mstatus)?;
        writer.u64((*snapshot).context.mepc)?;
        writer.u64((*snapshot).context.mcause)?;
        writer.u64((*snapshot).context.mtval)?;
        let mut source_length = 0usize;
        for index in 0..(*snapshot).source_count {
            source_length += (*snapshot).source_lengths[index];
            if index + 1 < (*snapshot).source_count {
                source_length += 1;
            }
        }
        writer.u32(source_length as u32)?;
        let symbol_count = (*snapshot)
            .symbols
            .iter()
            .filter(|symbol| symbol.enabled)
            .count();
        writer.u32(symbol_count as u32)?;
        for index in 0..(*snapshot).source_count {
            writer
                .bytes(&(&(*snapshot).source_lines[index])[..(*snapshot).source_lengths[index]])?;
            if index + 1 < (*snapshot).source_count {
                writer.bytes(b"\n")?;
            }
        }
        for symbol in &(*snapshot).symbols {
            if symbol.enabled {
                writer.u64(symbol.address)?;
                writer.u16(symbol.length as u16)?;
                writer.bytes(&symbol.name[..symbol.length])?;
            }
        }
    }
    Some(writer.offset)
}

struct MetadataWriter {
    offset: usize,
}

impl MetadataWriter {
    fn bytes(&mut self, bytes: &[u8]) -> Option<()> {
        let end = self.offset.checked_add(bytes.len())?;
        if end > MAX_METADATA_BYTES {
            return None;
        }
        unsafe {
            SNAPSHOT_METADATA[self.offset..end].copy_from_slice(bytes);
        }
        self.offset = end;
        Some(())
    }

    fn u16(&mut self, value: u16) -> Option<()> {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Option<()> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Option<()> {
        self.bytes(&value.to_le_bytes())
    }
}

fn snapshot_manifest() {
    let snapshot = core::ptr::addr_of!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        guest_error(b"GUEST-SNAPSHOT-002", b"no snapshot or project is saved");
        return;
    }
    let source_count =
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).source_count)) };
    let workspace_crc = snapshot_crc32(true);
    let data_crc = snapshot_crc32(false);
    uart_write(
        "snapshot-manifest format=RVSNAP01 workspace-size=65536 data-size=1048576 source-lines=",
    );
    uart_decimal(source_count as u64);
    uart_write(" workspace-crc32=0x");
    uart_hex(u64::from(workspace_crc));
    uart_write(" data-crc32=0x");
    uart_hex(u64::from(data_crc));
    uart_write(" chunk-max=4096\r\n");
}

fn snapshot_crc32(workspace: bool) -> u32 {
    let snapshot = core::ptr::addr_of!(GUEST_SNAPSHOT);
    let bytes = unsafe {
        if workspace {
            &(&(*snapshot).workspace)[..]
        } else {
            &(&(*snapshot).data)[..]
        }
    };
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn snapshot_region_name(input: &[u8]) -> Option<bool> {
    match input {
        b"workspace" => Some(true),
        b"data" => Some(false),
        _ => None,
    }
}

fn snapshot_dump(argument: &[u8]) {
    let Some((region_bytes, rest)) = split_token_space(argument) else {
        guest_error(
            b"GUEST-SNAPSHOT-003",
            b"dump expects <workspace|data> <offset> <length>",
        );
        return;
    };
    let Some(workspace) = snapshot_region_name(region_bytes) else {
        guest_error(
            b"GUEST-SNAPSHOT-004",
            b"snapshot region must be workspace or data",
        );
        return;
    };
    let Some((offset_bytes, length_bytes)) = split_token_space(rest) else {
        guest_error(
            b"GUEST-SNAPSHOT-003",
            b"dump expects <workspace|data> <offset> <length>",
        );
        return;
    };
    let Some(offset) = parse_decimal(offset_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-005", b"snapshot offset must be decimal");
        return;
    };
    let Some(length) = parse_decimal(length_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-006", b"snapshot length must be decimal");
        return;
    };
    if !snapshot_valid_chunk(workspace, offset, length) || length == 0 || length > MAX_SNAPSHOT_DUMP
    {
        guest_error(
            b"GUEST-SNAPSHOT-007",
            b"snapshot chunk must be 1..4096 bytes inside its region",
        );
        return;
    }
    let snapshot = core::ptr::addr_of!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        guest_error(b"GUEST-SNAPSHOT-002", b"no snapshot or project is saved");
        return;
    }
    let bytes = unsafe {
        if workspace {
            &(&(*snapshot).workspace)[offset as usize..(offset + length) as usize]
        } else {
            &(&(*snapshot).data)[offset as usize..(offset + length) as usize]
        }
    };
    uart_write("snapshot-chunk ");
    uart_bytes(region_bytes);
    uart_write(" offset=");
    uart_decimal(offset);
    uart_write(" length=");
    uart_decimal(length);
    uart_write(" hex=");
    for byte in bytes {
        uart_hex_byte(*byte);
    }
    uart_write("\r\n");
}

fn snapshot_patch(argument: &[u8]) {
    let Some((region_bytes, rest)) = split_token_space(argument) else {
        guest_error(
            b"GUEST-SNAPSHOT-008",
            b"patch expects <workspace|data> <offset> <hex-bytes>",
        );
        return;
    };
    let Some(workspace) = snapshot_region_name(region_bytes) else {
        guest_error(
            b"GUEST-SNAPSHOT-004",
            b"snapshot region must be workspace or data",
        );
        return;
    };
    let Some((offset_bytes, hex_bytes)) = split_token_space(rest) else {
        guest_error(
            b"GUEST-SNAPSHOT-008",
            b"patch expects <workspace|data> <offset> <hex-bytes>",
        );
        return;
    };
    let Some(offset) = parse_decimal(offset_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-005", b"snapshot offset must be decimal");
        return;
    };
    let mut bytes = [0u8; MAX_EDIT_BYTES];
    let Some(length) = parse_hex_bytes(hex_bytes, &mut bytes) else {
        guest_error(
            b"GUEST-SNAPSHOT-009",
            b"snapshot patch expects 1..32 hexadecimal bytes",
        );
        return;
    };
    if !snapshot_valid_chunk(workspace, offset, length as u64) {
        guest_error(
            b"GUEST-SNAPSHOT-007",
            b"snapshot chunk is outside its region",
        );
        return;
    }
    let snapshot = core::ptr::addr_of_mut!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        guest_error(b"GUEST-SNAPSHOT-002", b"no snapshot or project is saved");
        return;
    }
    unsafe {
        if workspace {
            (&mut (*snapshot).workspace)[offset as usize..offset as usize + length]
                .copy_from_slice(&bytes[..length]);
        } else {
            (&mut (*snapshot).data)[offset as usize..offset as usize + length]
                .copy_from_slice(&bytes[..length]);
        }
    }
    uart_write("snapshot chunk patched ");
    uart_bytes(region_bytes);
    uart_write(" offset=");
    uart_decimal(offset);
    uart_write(" length=");
    uart_decimal(length as u64);
    uart_write("\r\n");
}

fn snapshot_patch_binary(argument: &[u8]) {
    let Some((region_bytes, rest)) = split_token_space(argument) else {
        guest_error(
            b"GUEST-SNAPSHOT-010",
            b"patchbin expects <workspace|data> <offset> <length> followed by raw bytes",
        );
        return;
    };
    let Some(workspace) = snapshot_region_name(region_bytes) else {
        guest_error(
            b"GUEST-SNAPSHOT-004",
            b"snapshot region must be workspace or data",
        );
        return;
    };
    let Some((offset_bytes, length_bytes)) = split_token_space(rest) else {
        guest_error(
            b"GUEST-SNAPSHOT-010",
            b"patchbin expects <workspace|data> <offset> <length> followed by raw bytes",
        );
        return;
    };
    let Some(offset) = parse_decimal(offset_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-005", b"snapshot offset must be decimal");
        return;
    };
    let Some(length) = parse_decimal(length_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-006", b"snapshot length must be decimal");
        return;
    };
    if length == 0 || length > MAX_SNAPSHOT_DUMP || !snapshot_valid_chunk(workspace, offset, length)
    {
        guest_error(
            b"GUEST-SNAPSHOT-007",
            b"snapshot binary chunk must be 1..4096 bytes inside its region",
        );
        return;
    }
    let snapshot = core::ptr::addr_of_mut!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        guest_error(b"GUEST-SNAPSHOT-002", b"no snapshot or project is saved");
        return;
    }
    uart_write("snapshot binary ready\r\n");
    for index in 0..length as usize {
        unsafe {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(SNAPSHOT_BINARY_PATCH[index]),
                uart_get(),
            );
        }
    }
    unsafe {
        if workspace {
            (&mut (*snapshot).workspace)[offset as usize..offset as usize + length as usize]
                .copy_from_slice(&SNAPSHOT_BINARY_PATCH[..length as usize]);
        } else {
            (&mut (*snapshot).data)[offset as usize..offset as usize + length as usize]
                .copy_from_slice(&SNAPSHOT_BINARY_PATCH[..length as usize]);
        }
    }
    uart_write("snapshot binary chunk patched ");
    uart_bytes(region_bytes);
    uart_write(" offset=");
    uart_decimal(offset);
    uart_write(" length=");
    uart_decimal(length);
    uart_write("\r\n");
}

fn snapshot_patch_rle(argument: &[u8]) {
    let Some((region_bytes, rest)) = split_token_space(argument) else {
        guest_error(
            b"GUEST-SNAPSHOT-011",
            b"patchrle expects <workspace|data> <offset> <raw-length> <encoded-length>",
        );
        return;
    };
    let Some(workspace) = snapshot_region_name(region_bytes) else {
        guest_error(
            b"GUEST-SNAPSHOT-004",
            b"snapshot region must be workspace or data",
        );
        return;
    };
    let Some((offset_bytes, rest)) = split_token_space(rest) else {
        guest_error(
            b"GUEST-SNAPSHOT-011",
            b"patchrle expects <workspace|data> <offset> <raw-length> <encoded-length>",
        );
        return;
    };
    let Some((raw_length_bytes, encoded_length_bytes)) = split_token_space(rest) else {
        guest_error(
            b"GUEST-SNAPSHOT-011",
            b"patchrle expects <workspace|data> <offset> <raw-length> <encoded-length>",
        );
        return;
    };
    let Some(offset) = parse_decimal(offset_bytes) else {
        guest_error(b"GUEST-SNAPSHOT-005", b"snapshot offset must be decimal");
        return;
    };
    let Some(raw_length) = parse_decimal(raw_length_bytes) else {
        guest_error(
            b"GUEST-SNAPSHOT-012",
            b"snapshot raw length must be decimal",
        );
        return;
    };
    let Some(encoded_length) = parse_decimal(encoded_length_bytes) else {
        guest_error(
            b"GUEST-SNAPSHOT-012",
            b"snapshot encoded length must be decimal",
        );
        return;
    };
    if raw_length == 0
        || raw_length > MAX_SNAPSHOT_DUMP
        || encoded_length == 0
        || encoded_length > MAX_SNAPSHOT_DUMP
        || !snapshot_valid_chunk(workspace, offset, raw_length)
        || encoded_length >= raw_length
        || encoded_length % 2 != 0
    {
        guest_error(
            b"GUEST-SNAPSHOT-013",
            b"invalid RLE chunk lengths or region bounds",
        );
        return;
    }
    let snapshot = core::ptr::addr_of_mut!(GUEST_SNAPSHOT);
    let valid = unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*snapshot).valid)) };
    if !valid {
        guest_error(b"GUEST-SNAPSHOT-002", b"no snapshot or project is saved");
        return;
    }
    uart_write("snapshot binary ready\r\n");
    for index in 0..encoded_length as usize {
        unsafe {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(SNAPSHOT_BINARY_COMPRESSED[index]),
                uart_get(),
            );
        }
    }
    let mut encoded_index = 0usize;
    let mut decoded_length = 0usize;
    while encoded_index < encoded_length as usize {
        let run_length = unsafe { SNAPSHOT_BINARY_COMPRESSED[encoded_index] as usize };
        let byte = unsafe { SNAPSHOT_BINARY_COMPRESSED[encoded_index + 1] };
        if decoded_length + run_length > raw_length as usize {
            guest_error(b"GUEST-SNAPSHOT-013", b"RLE expands beyond raw length");
            return;
        }
        let end = decoded_length + run_length;
        unsafe {
            SNAPSHOT_BINARY_PATCH[decoded_length..end].fill(byte);
        }
        decoded_length = end;
        encoded_index += 2;
    }
    if decoded_length != raw_length as usize {
        guest_error(b"GUEST-SNAPSHOT-013", b"RLE raw length mismatch");
        return;
    }
    unsafe {
        if workspace {
            (&mut (*snapshot).workspace)[offset as usize..offset as usize + raw_length as usize]
                .copy_from_slice(&SNAPSHOT_BINARY_PATCH[..raw_length as usize]);
        } else {
            (&mut (*snapshot).data)[offset as usize..offset as usize + raw_length as usize]
                .copy_from_slice(&SNAPSHOT_BINARY_PATCH[..raw_length as usize]);
        }
    }
    uart_write("snapshot binary chunk patched ");
    uart_bytes(region_bytes);
    uart_write(" offset=");
    uart_decimal(offset);
    uart_write(" length=");
    uart_decimal(raw_length);
    uart_write(" encoding=rle\r\n");
}

fn snapshot_valid_chunk(workspace: bool, offset: u64, length: u64) -> bool {
    let capacity = if workspace {
        TARGET_WORKSPACE_BYTES as u64
    } else {
        TARGET_DATA_BYTES as u64
    };
    offset
        .checked_add(length)
        .is_some_and(|end| end <= capacity)
}

fn assemble_source_buffer(
    context: *mut TargetContext,
    address: u64,
    lines: &[[u8; COMMAND_CAPACITY]; MAX_SOURCE_LINES],
    lengths: &[usize; MAX_SOURCE_LINES],
    count: usize,
) -> ! {
    let mut staged_symbols = [GuestSymbol::empty(); MAX_SYMBOLS];
    let mut instruction_count = 0usize;
    for index in 0..count {
        let line = &lines[index][..lengths[index]];
        if let Some(label) = line.strip_suffix(b":") {
            let line_address = address + (instruction_count as u64) * 4;
            let Some(symbol) = make_symbol(label, line_address) else {
                guest_source_error(
                    index + 1,
                    b"GUEST-ASM-003",
                    b"invalid, duplicate or too many source labels",
                );
                monitor_loop(context);
            };
            if staged_symbols
                .iter()
                .any(|slot| slot.enabled && &slot.name[..slot.length] == label)
            {
                guest_source_error(
                    index + 1,
                    b"GUEST-ASM-003",
                    b"invalid, duplicate or too many source labels",
                );
                monitor_loop(context);
            }
            let Some(slot) = staged_symbols.iter_mut().find(|slot| !slot.enabled) else {
                guest_source_error(
                    index + 1,
                    b"GUEST-ASM-003",
                    b"invalid, duplicate or too many source labels",
                );
                monitor_loop(context);
            };
            *slot = symbol;
        } else {
            instruction_count += 1;
        }
    }
    if instruction_count == 0 {
        guest_error(b"GUEST-ASM-004", b"source program contains no instructions");
        monitor_loop(context);
    }
    let Some(end_address) = address.checked_add((instruction_count as u64) * 4) else {
        guest_error(b"GUEST-ASM-005", b"source program address overflows");
        monitor_loop(context);
    };
    if end_address > TARGET_RAM_END {
        guest_error(
            b"GUEST-ASM-006",
            b"source program does not fit in target RAM",
        );
        monitor_loop(context);
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
            guest_source_error(
                index + 1,
                b"GUEST-ASM-007",
                b"source program exceeds target workspace",
            );
            monitor_loop(context);
        }
        let Some(word) = parse_source_instruction(line, line_address, &staged_symbols) else {
            guest_source_error(
                index + 1,
                b"GUEST-ASM-008",
                b"supports integer/control, ld/sd, fadd.s/fadd.d or fmv syntax",
            );
            monitor_loop(context);
        };
        if permanent_breakpoint_at(line_address).is_some() || temporary_breakpoint_at(line_address)
        {
            guest_source_error(
                index + 1,
                b"GUEST-ASM-009",
                b"source overlaps an active breakpoint",
            );
            monitor_loop(context);
        }
        words[word_count] = word;
        word_addresses[word_count] = line_address;
        word_count += 1;
    }
    clear_memory_undo();
    for index in 0..word_count {
        if !target_store32(word_addresses[index], words[index]) {
            guest_error(b"GUEST-ASM-010", b"cannot write assembled source program");
            monitor_loop(context);
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
    uart_write("assembled source: ");
    uart_decimal(word_count as u64);
    uart_write(" instruction(s) at 0x");
    uart_hex(address);
    uart_write("\r\n");
    monitor_loop(context);
}

fn parse_source_instruction(
    source: &[u8],
    address: u64,
    symbols: &[GuestSymbol; MAX_SYMBOLS],
) -> Option<u32> {
    if let Some(operands) = source.strip_prefix(b"addi ") {
        return parse_addi_operands(operands, symbols);
    }
    if let Some(operands) = source.strip_prefix(b"lui ") {
        return parse_u_operands("lui", operands);
    }
    if let Some(operands) = source.strip_prefix(b"auipc ") {
        return parse_u_operands("auipc", operands);
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
    for (mnemonic, rd_float, rs1_float) in [
        ("fmv.w.x", true, false),
        ("fmv.x.w", false, true),
        ("fmv.d.x", true, false),
        ("fmv.x.d", false, true),
    ] {
        if let Some(operands) = source.strip_prefix(mnemonic.as_bytes()) {
            if let Some(operands) = operands.strip_prefix(b" ") {
                return parse_float_move_operands(mnemonic, operands, rd_float, rs1_float);
            }
        }
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

fn parse_u_operands(mnemonic: &str, operands: &[u8]) -> Option<u32> {
    let (rd_bytes, immediate_bytes) = split_once_comma(operands)?;
    let rd = parse_register(rd_bytes.trim_ascii())?;
    let immediate = parse_hex(immediate_bytes.trim_ascii())
        .or_else(|| parse_decimal(immediate_bytes.trim_ascii()))?;
    if mnemonic == "lui" {
        luna_isa_core::encode_lui(rd, u32::try_from(immediate).ok()?)
    } else {
        luna_isa_core::encode_auipc(rd, u32::try_from(immediate).ok()?)
    }
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

fn parse_float_move_operands(
    mnemonic: &str,
    operands: &[u8],
    rd_float: bool,
    rs1_float: bool,
) -> Option<u32> {
    let (rd_bytes, rs1_bytes) = split_once_comma(operands)?;
    let rd = if rd_float {
        parse_float_register(rd_bytes.trim_ascii())?
    } else {
        parse_register(rd_bytes.trim_ascii())?
    };
    let rs1 = if rs1_float {
        parse_float_register(rs1_bytes.trim_ascii())?
    } else {
        parse_register(rs1_bytes.trim_ascii())?
    };
    let opcode = GENERATED_OPCODES
        .iter()
        .find(|opcode| opcode.mnemonic == mnemonic)?;
    Some(opcode.match_value | ((rs1 as u32) << 15) | ((rd as u32) << 7))
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

    resume_single_step(context);
}

fn run_target(context: *mut TargetContext, argument: &[u8]) -> ! {
    let context = unsafe { &mut *context };
    if context.mcause != StopReason::Breakpoint as u64 {
        guest_error(b"GUEST-RUN-001", b"target is not stopped at a breakpoint");
        monitor_loop(context);
    }
    let Some(budget) = parse_decimal(argument.trim_ascii()) else {
        guest_error(b"GUEST-RUN-002", b"run expects a decimal instruction count");
        monitor_loop(context);
    };
    if budget == 0 || budget > 100_000 {
        guest_error(b"GUEST-RUN-003", b"run count must be between 1 and 100000");
        monitor_loop(context);
    }
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(RUN_REMAINING), budget);
    }
    resume_single_step(context);
}

fn resume_single_step(context: *mut TargetContext) -> ! {
    let context = unsafe { &mut *context };

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
    if let Some((slot, address, width)) =
        watchpoint_for_instruction(context, instruction_pc, instruction_word)
    {
        unsafe {
            core::ptr::write_volatile(core::ptr::addr_of_mut!(RUN_REMAINING), 0);
        }
        uart_write("watchpoint #");
        uart_decimal((slot + 1) as u64);
        uart_write(" hit at pc=0x");
        uart_hex(instruction_pc);
        uart_write(" address=0x");
        uart_hex(address);
        uart_write(" width=");
        uart_decimal(width);
        uart_write("\r\n");
        monitor_loop(context);
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

fn watchpoint_for_instruction(
    context: &TargetContext,
    _pc: u64,
    word: u32,
) -> Option<(usize, u64, u64)> {
    let opcode = word & 0x7f;
    let funct3 = (word >> 12) & 0x7;
    let (kind, immediate) = match (opcode, funct3) {
        (0x03, 0b011) => {
            let immediate = (word as i32 >> 20) as i64 as u64;
            (GuestWatchKind::Read, immediate)
        }
        (0x23, 0b011) => {
            let immediate = (((word >> 25) & 0x7f) << 5) | ((word >> 7) & 0x1f);
            let immediate = ((immediate as i32) << 20 >> 20) as i64 as u64;
            (GuestWatchKind::Write, immediate)
        }
        _ => return None,
    };
    let base = context.x[((word >> 15) & 0x1f) as usize];
    let address = base.wrapping_add(immediate);
    let width = 8;
    let access_end = address.checked_add(width)?;
    for index in 0..MAX_WATCHPOINTS {
        let watchpoint =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(WATCHPOINTS[index])) };
        if !watchpoint.enabled {
            continue;
        }
        let watch_end = watchpoint.address.checked_add(watchpoint.width)?;
        let kind_matches = matches!(watchpoint.kind, GuestWatchKind::Any)
            || matches!(
                (watchpoint.kind, kind),
                (GuestWatchKind::Read, GuestWatchKind::Read)
            )
            || matches!(
                (watchpoint.kind, kind),
                (GuestWatchKind::Write, GuestWatchKind::Write)
            );
        if kind_matches && address < watch_end && watchpoint.address < access_end {
            return Some((index, address, width));
        }
    }
    None
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

fn add_watchpoint(argument: &[u8], kind: GuestWatchKind) {
    let Some((address_bytes, width_bytes)) = split_token_space(argument) else {
        guest_error(
            b"GUEST-WATCH-001",
            b"watch expects <hex-address> <decimal-width>",
        );
        return;
    };
    let Some(address) = parse_hex(address_bytes) else {
        guest_error(b"GUEST-WATCH-002", b"watch address must be hexadecimal");
        return;
    };
    let Some(width) = parse_decimal(width_bytes.trim_ascii()) else {
        guest_error(b"GUEST-WATCH-003", b"watch width must be decimal");
        return;
    };
    let Some(end) = address.checked_add(width) else {
        guest_error(b"GUEST-WATCH-004", b"watch range overflows");
        return;
    };
    if width == 0 || width > 8 || address < TARGET_RAM_START || end > TARGET_RAM_END {
        guest_error(
            b"GUEST-WATCH-005",
            b"watch range must be 1..8 bytes inside target RAM",
        );
        return;
    }
    let mut slot = None;
    for index in 0..MAX_WATCHPOINTS {
        let watchpoint =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(WATCHPOINTS[index])) };
        if !watchpoint.enabled {
            slot = Some(index);
            break;
        }
    }
    let Some(slot) = slot else {
        guest_error(b"GUEST-WATCH-006", b"watchpoint table is full");
        return;
    };
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(WATCHPOINTS[slot]),
            GuestWatchpoint {
                address,
                width,
                kind,
                enabled: true,
            },
        );
    }
    uart_write("watchpoint #");
    uart_decimal((slot + 1) as u64);
    uart_write(" set at 0x");
    uart_hex(address);
    uart_write(" width=");
    uart_decimal(width);
    uart_write(" mode=");
    uart_write(match kind {
        GuestWatchKind::Read => "read",
        GuestWatchKind::Write => "write",
        GuestWatchKind::Any => "access",
    });
    uart_write("\r\n");
}

fn delete_watchpoint(argument: &[u8]) {
    let Some(number) = parse_decimal(argument.trim_ascii()) else {
        guest_error(b"GUEST-WATCH-007", b"watch number must be decimal");
        return;
    };
    if number == 0 || number > MAX_WATCHPOINTS as u64 {
        guest_error(b"GUEST-WATCH-008", b"watch number is out of range");
        return;
    }
    let slot = (number - 1) as usize;
    let watchpoint = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(WATCHPOINTS[slot])) };
    if !watchpoint.enabled {
        guest_error(b"GUEST-WATCH-009", b"watchpoint is not enabled");
        return;
    }
    unsafe {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!(WATCHPOINTS[slot]),
            GuestWatchpoint::disabled(),
        );
    }
    uart_write("watchpoint #");
    uart_decimal(number);
    uart_write(" deleted\r\n");
}

fn print_watchpoints() {
    uart_write("watchpoints:\r\n");
    let mut found = false;
    for index in 0..MAX_WATCHPOINTS {
        let watchpoint =
            unsafe { core::ptr::read_volatile(core::ptr::addr_of!(WATCHPOINTS[index])) };
        if watchpoint.enabled {
            found = true;
            uart_write("  #");
            uart_decimal((index + 1) as u64);
            uart_write(" addr=0x");
            uart_hex(watchpoint.address);
            uart_write(" width=");
            uart_decimal(watchpoint.width);
            uart_write(" mode=");
            uart_write(match watchpoint.kind {
                GuestWatchKind::Read => "read",
                GuestWatchKind::Write => "write",
                GuestWatchKind::Any => "access",
            });
            uart_write("\r\n");
        }
    }
    if !found {
        uart_write("  none\r\n");
    }
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

fn valid_target_data_range(address: u64, length: usize) -> bool {
    address >= target_data_start()
        && address
            .checked_add(length as u64)
            .is_some_and(|end| end <= target_data_end())
}

fn target_workspace_start() -> u64 {
    core::ptr::addr_of!(_target_workspace_start) as u64
}

fn target_workspace_end() -> u64 {
    core::ptr::addr_of!(_target_workspace_end) as u64
}

fn target_data_start() -> u64 {
    core::ptr::addr_of!(_target_data_start) as u64
}

fn target_data_end() -> u64 {
    core::ptr::addr_of!(_target_data_end) as u64
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
    static _target_data_start: u8;
    static _target_data_end: u8;
}

fn uart_write(text: &str) {
    for byte in text.bytes() {
        uart_put(byte);
    }
}

fn uart_init() {
    unsafe {
        core::ptr::write_volatile(
            (UART_BASE + UART_FCR) as *mut u8,
            UART_FCR_ENABLE_FIFO | UART_FCR_TRIGGER_1,
        );
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
    unsafe {
        if UART_RX_INDEX == UART_RX_LENGTH {
            UART_RX_INDEX = 0;
            UART_RX_LENGTH = 0;
            while core::ptr::read_volatile((UART_BASE + UART_LSR) as *const u8)
                & UART_LSR_DATA_READY
                == 0
            {}
            while UART_RX_LENGTH < UART_RX_BUFFER_CAPACITY
                && core::ptr::read_volatile((UART_BASE + UART_LSR) as *const u8)
                    & UART_LSR_DATA_READY
                    != 0
            {
                UART_RX_BUFFER[UART_RX_LENGTH] = core::ptr::read_volatile(UART_BASE as *const u8);
                UART_RX_LENGTH += 1;
            }
        }
        let byte = UART_RX_BUFFER[UART_RX_INDEX];
        UART_RX_INDEX += 1;
        byte
    }
}
