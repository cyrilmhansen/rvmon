#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

use luna_target_api::StopReason;
use luna_target_api::TargetCapabilities;
use luna_target_api::TargetContext;

const UART_BASE: usize = 0x1000_0000;
const UART_LSR: usize = 5;
const UART_LSR_DATA_READY: u8 = 1 << 0;
const UART_LSR_EMPTY: u8 = 1 << 5;
const COMMAND_CAPACITY: usize = 32;

global_asm!(include_str!("entry.S"));

static mut CONTEXT: TargetContext = TargetContext::empty();
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
    let context = unsafe { &*context };
    uart_write("trap: ");
    if context.mcause == StopReason::Breakpoint as u64 {
        uart_write("breakpoint");
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
            b"step" | b"s" | b"continue" | b"c" => resume_after_breakpoint(context),
            b"quit" | b"exit" | b"q" => {
                uart_write("bye\r\n");
            }
            b"" => {}
            _ => uart_write("error: unknown command; use help\r\n"),
        }
    }
}

fn print_help() {
    uart_write("help/?  regs/registers  step/s  continue/c  quit/q\r\n");
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

fn resume_after_breakpoint(context: *mut TargetContext) -> ! {
    let context = unsafe { &mut *context };
    if context.mcause != StopReason::Breakpoint as u64 {
        uart_write("error: target is not stopped at a breakpoint\r\n");
        monitor_loop(context);
    }
    let next_pc = match context.pc.checked_add(4) {
        Some(next_pc) => next_pc,
        None => {
            uart_write("error: breakpoint pc overflow\r\n");
            monitor_loop(context);
        }
    };
    context.pc = next_pc;
    context.mepc = next_pc;
    unsafe { resume_user(context as *mut TargetContext) }
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
