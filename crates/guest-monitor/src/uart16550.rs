//! Minimal 16550A driver for the QEMU `virt` machine.
//!
//! This module intentionally contains the MMIO boundary and the software RX
//! queue in one place.  Interrupt delivery is used while U-mode runs;
//! polling remains as a safe fallback at environment-call boundaries and in
//! the M-mode command loop.

const UART_BASE: usize = 0x1000_0000;

const UART_RX: usize = 0;
const UART_TX: usize = 0;
const UART_IER: usize = 1;
const UART_FCR: usize = 2;
const UART_LCR: usize = 3;
const UART_MCR: usize = 4;
const UART_LSR: usize = 5;
const UART_DLL: usize = 0;
const UART_DLM: usize = 1;

const UART_IER_DISABLE: u8 = 0;
const UART_IER_RLSI: u8 = 1 << 2;
const UART_IER_RDI: u8 = 1 << 0;
const UART_FCR_ENABLE_FIFO: u8 = 1 << 0;
const UART_FCR_CLEAR_RX: u8 = 1 << 1;
const UART_FCR_CLEAR_TX: u8 = 1 << 2;
const UART_FCR_TRIGGER_1: u8 = 0;
const UART_LCR_DLAB: u8 = 1 << 7;
const UART_LCR_8N1: u8 = 0x03;
const UART_MCR_DTR: u8 = 1 << 0;
const UART_MCR_RTS: u8 = 1 << 1;
const UART_MCR_OUT2: u8 = 1 << 3;
const UART_LSR_DATA_READY: u8 = 1 << 0;
const UART_LSR_OVERRUN: u8 = 1 << 1;
const UART_LSR_PARITY: u8 = 1 << 2;
const UART_LSR_FRAMING: u8 = 1 << 3;
const UART_LSR_BREAK: u8 = 1 << 4;
const UART_LSR_EMPTY: u8 = 1 << 5;

// QEMU's serial device uses a 115200 baud base.  Divisor 12 is its documented
// reset rate (9600 baud) and keeps the guest-side contract stable for the
// current stdio/TCP chardev paths.
const UART_BAUD_DIVISOR: u16 = 12;
const RX_CAPACITY: usize = 4096;

static mut RX_BUFFER: [u8; RX_CAPACITY] = [0; RX_CAPACITY];
static mut RX_HEAD: usize = 0;
static mut RX_TAIL: usize = 0;
static mut RX_COUNT: usize = 0;
static mut RX_HARDWARE_OVERRUNS: u64 = 0;
static mut RX_SOFTWARE_DROPS: u64 = 0;
static mut RX_PARITY_ERRORS: u64 = 0;
static mut RX_FRAMING_ERRORS: u64 = 0;
static mut RX_BREAKS: u64 = 0;
static mut RX_CTRL_C_REQUESTS: u64 = 0;
static mut RX_INTERRUPT_SERVICES: u64 = 0;
static mut RX_BREAK_REQUESTED: bool = false;

#[derive(Clone, Copy)]
pub struct Stats {
    pub queued: usize,
    pub hardware_overruns: u64,
    pub software_drops: u64,
    pub parity_errors: u64,
    pub framing_errors: u64,
    pub breaks: u64,
    pub ctrl_c_requests: u64,
    pub interrupt_services: u64,
}

pub fn init() {
    unsafe {
        write_reg(UART_IER, UART_IER_DISABLE);

        // Program 8N1 and an explicit divisor through the DLAB window.
        write_reg(UART_LCR, UART_LCR_DLAB);
        write_reg(UART_DLL, (UART_BAUD_DIVISOR & 0xff) as u8);
        write_reg(UART_DLM, (UART_BAUD_DIVISOR >> 8) as u8);
        write_reg(UART_LCR, UART_LCR_8N1);

        // Enable and clear both FIFOs.  Trigger level 1 is deliberate for
        // the polling phase: it minimizes latency and makes host backpressure
        // visible as early as possible.
        write_reg(
            UART_FCR,
            UART_FCR_ENABLE_FIFO | UART_FCR_CLEAR_RX | UART_FCR_CLEAR_TX | UART_FCR_TRIGGER_1,
        );

        // Advertise that the guest is ready to receive.  QEMU may expose
        // these modem-control bits through a real chardev, but the driver does
        // not rely on them for correctness.
        write_reg(UART_MCR, UART_MCR_DTR | UART_MCR_RTS | UART_MCR_OUT2);

        RX_HEAD = 0;
        RX_TAIL = 0;
        RX_COUNT = 0;
        RX_HARDWARE_OVERRUNS = 0;
        RX_SOFTWARE_DROPS = 0;
        RX_PARITY_ERRORS = 0;
        RX_FRAMING_ERRORS = 0;
        RX_BREAKS = 0;
        RX_CTRL_C_REQUESTS = 0;
        RX_INTERRUPT_SERVICES = 0;
        RX_BREAK_REQUESTED = false;
    }
}

pub fn put(byte: u8) {
    unsafe {
        // M-mode command processing masks external interrupts because the
        // trap frame belongs to the stopped U-mode target.  Drain the RX FIFO
        // opportunistically while producing output so a host that pipelines
        // commands cannot overflow the 16-byte hardware FIFO.
        poll_rx();
        while read_reg(UART_LSR) & UART_LSR_EMPTY == 0 {
            core::hint::spin_loop();
        }
        write_reg(UART_TX, byte);
        poll_rx();
    }
}

pub fn get() -> u8 {
    loop {
        poll_rx();
        if let Some(byte) = pop_rx() {
            return byte;
        }
        core::hint::spin_loop();
    }
}

pub fn try_get() -> Option<u8> {
    poll_rx();
    pop_rx()
}

pub fn peek() -> Option<u8> {
    poll_rx();
    unsafe {
        if RX_COUNT == 0 {
            None
        } else {
            Some(RX_BUFFER[RX_TAIL])
        }
    }
}

pub fn enable_receive_interrupts() {
    unsafe { write_reg(UART_IER, UART_IER_RLSI | UART_IER_RDI) }
}

pub fn disable_receive_interrupts() {
    unsafe { write_reg(UART_IER, UART_IER_DISABLE) }
}

pub fn take_break_request() -> bool {
    unsafe {
        let requested = RX_BREAK_REQUESTED;
        RX_BREAK_REQUESTED = false;
        requested
    }
}

pub fn service_interrupt() {
    unsafe { RX_INTERRUPT_SERVICES = RX_INTERRUPT_SERVICES.saturating_add(1) }
    poll_rx();
}

pub fn stats() -> Stats {
    unsafe {
        Stats {
            queued: RX_COUNT,
            hardware_overruns: RX_HARDWARE_OVERRUNS,
            software_drops: RX_SOFTWARE_DROPS,
            parity_errors: RX_PARITY_ERRORS,
            framing_errors: RX_FRAMING_ERRORS,
            breaks: RX_BREAKS,
            ctrl_c_requests: RX_CTRL_C_REQUESTS,
            interrupt_services: RX_INTERRUPT_SERVICES,
        }
    }
}

fn poll_rx() {
    unsafe {
        loop {
            let status = read_reg(UART_LSR);
            if status & UART_LSR_OVERRUN != 0 {
                RX_HARDWARE_OVERRUNS = RX_HARDWARE_OVERRUNS.saturating_add(1);
            }
            if status & UART_LSR_PARITY != 0 {
                RX_PARITY_ERRORS = RX_PARITY_ERRORS.saturating_add(1);
            }
            if status & UART_LSR_FRAMING != 0 {
                RX_FRAMING_ERRORS = RX_FRAMING_ERRORS.saturating_add(1);
            }
            if status & UART_LSR_BREAK != 0 {
                RX_BREAKS = RX_BREAKS.saturating_add(1);
            }
            if status & UART_LSR_DATA_READY == 0 {
                return;
            }

            let byte = read_reg(UART_RX);
            if byte == 3 {
                RX_CTRL_C_REQUESTS = RX_CTRL_C_REQUESTS.saturating_add(1);
                RX_BREAK_REQUESTED = true;
                continue;
            }
            if RX_COUNT == RX_CAPACITY {
                RX_SOFTWARE_DROPS = RX_SOFTWARE_DROPS.saturating_add(1);
                continue;
            }
            RX_BUFFER[RX_HEAD] = byte;
            RX_HEAD = (RX_HEAD + 1) % RX_CAPACITY;
            RX_COUNT += 1;
        }
    }
}

fn pop_rx() -> Option<u8> {
    unsafe {
        if RX_COUNT == 0 {
            return None;
        }
        let byte = RX_BUFFER[RX_TAIL];
        RX_TAIL = (RX_TAIL + 1) % RX_CAPACITY;
        RX_COUNT -= 1;
        Some(byte)
    }
}

unsafe fn read_reg(offset: usize) -> u8 {
    // SAFETY: the QEMU `virt` machine maps the 16550A UART at UART_BASE and
    // all accesses are byte-sized, as required by the device model.
    unsafe { core::ptr::read_volatile((UART_BASE + offset) as *const u8) }
}

unsafe fn write_reg(offset: usize, value: u8) {
    // SAFETY: the QEMU `virt` machine maps the 16550A UART at UART_BASE and
    // all accesses are byte-sized, as required by the device model.
    unsafe { core::ptr::write_volatile((UART_BASE + offset) as *mut u8, value) }
}
