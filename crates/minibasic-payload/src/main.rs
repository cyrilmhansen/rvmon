#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod minibasic {
    include!("../../guest-monitor/src/minibasic.rs");
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(link_section = ".text.start")]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    minibasic::minibasic_entry()
}
