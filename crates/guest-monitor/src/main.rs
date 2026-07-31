#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::panic::PanicInfo;

use luna_target_api::Breakpoint;
use luna_target_api::StopReason;
use luna_target_api::TargetCapabilities;
use luna_target_api::TargetContext;

const UART_BASE: usize = 0x1000_0000;
const UART_LSR: usize = 5;
const UART_LSR_DATA_READY: u8 = 1 << 0;
const UART_LSR_EMPTY: u8 = 1 << 5;
const COMMAND_CAPACITY: usize = 32;
const TARGET_RAM_START: u64 = 0x8000_0000;
const TARGET_RAM_END: u64 = 0x8002_0000;
const EBREAK_WORD: u32 = 0x0010_0073;
const MAX_PERMANENT_BREAKPOINTS: usize = 4;

global_asm!(include_str!("entry.S"));

static mut CONTEXT: TargetContext = TargetContext::empty();
static mut TEMPORARY_BREAKPOINT: Breakpoint = Breakpoint::disabled();
static mut PERMANENT_BREAKPOINTS: [Breakpoint; MAX_PERMANENT_BREAKPOINTS] =
    [Breakpoint::disabled(); MAX_PERMANENT_BREAKPOINTS];
static mut STEPPED_PERMANENT_BREAKPOINT: u8 = u8::MAX;
static TARGET_STACK: [u8; 8192] = [0; 8192];

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
        "help/? regs/registers step/s continue/c break <addr> delete <n> info break quit/q\r\n",
    );
}

fn print_registers(context: *mut TargetContext) {
    let context = unsafe { &*context };
    uart_write("pc=0x");
    uart_hex(context.pc);
    uart_write(" x1=0x");
    uart_hex(context.x[1]);
    uart_write(" x2=0x");
    uart_hex(context.x[2]);
    uart_write(" fcsr=0x");
    uart_hex(u64::from(context.fcsr));
    uart_write("\r\n");
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
    let Some(address) = parse_hex(argument) else {
        uart_write("error: break expects an address such as 0x80000010\r\n");
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
}

fn uart_write(text: &str) {
    for byte in text.bytes() {
        uart_put(byte);
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
