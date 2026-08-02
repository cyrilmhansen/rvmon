//! Minimal SiFive PLIC access for one QEMU `virt` hart in M-mode.

const PLIC_BASE: usize = 0x0c00_0000;
const PLIC_PRIORITY_BASE: usize = 0x0000;
const PLIC_ENABLE_BASE: usize = 0x2000;
const PLIC_CONTEXT_BASE: usize = 0x20_0000;
const PLIC_CONTEXT_STRIDE: usize = 0x1000;
const MACHINE_CONTEXT: usize = 0;
const UART_IRQ: u32 = 10;

pub fn init() {
    unsafe {
        // Priority zero disables a source.  Priority one is sufficient for
        // the single UART source used by this monitor.
        write32(PLIC_PRIORITY_BASE + (UART_IRQ as usize * 4), 1);

        // Enable UART IRQ 10 in the one-hart M-mode enable word.
        write32(PLIC_ENABLE_BASE, 1u32 << UART_IRQ);

        // Accept all positive priorities for this context.
        write32(context_address(0), 0);
    }
}

pub fn claim() -> u32 {
    unsafe { read32(context_address(4)) }
}

pub fn complete(irq: u32) {
    if irq != 0 {
        unsafe { write32(context_address(4), irq) }
    }
}

fn context_address(offset: usize) -> usize {
    PLIC_CONTEXT_BASE + MACHINE_CONTEXT * PLIC_CONTEXT_STRIDE + offset
}

unsafe fn read32(offset: usize) -> u32 {
    // SAFETY: the QEMU `virt` machine maps the PLIC as little-endian 32-bit
    // MMIO at PLIC_BASE.
    unsafe { core::ptr::read_volatile((PLIC_BASE + offset) as *const u32) }
}

unsafe fn write32(offset: usize, value: u32) {
    // SAFETY: the QEMU `virt` machine maps the PLIC as little-endian 32-bit
    // MMIO at PLIC_BASE.
    unsafe { core::ptr::write_volatile((PLIC_BASE + offset) as *mut u32, value) }
}
