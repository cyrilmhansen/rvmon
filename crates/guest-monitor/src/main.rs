#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

use luna_target_api::TargetCapabilities;
use luna_target_api::TargetContext;

const UART_BASE: usize = 0x1000_0000;
const UART_LSR: usize = 5;
const UART_LSR_EMPTY: u8 = 1 << 5;

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
    uart_write("breakpoint test: entering U-mode\r\n");
    let target_stack = TARGET_STACK.as_ptr() as usize + TARGET_STACK.len();
    unsafe {
        enter_user(target_entry as *const () as usize, target_stack);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn target_entry() -> ! {
    uart_write("target: before ebreak\r\n");
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack));
    }
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_trap(context: *mut TargetContext) -> ! {
    let context = unsafe { &*context };
    uart_write("trap: ");
    if context.mcause == 3 {
        uart_write("breakpoint");
    } else {
        uart_write("unexpected");
    }
    uart_write(" pc=0x");
    uart_hex(context.mepc);
    uart_write("\r\nrvmonitor> ");
    loop {
        core::hint::spin_loop();
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
