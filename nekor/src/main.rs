#![cfg_attr(not(usermode), no_std)]
#![cfg_attr(not(usermode), no_main)]

use core::panic::PanicInfo;

#[cfg(usermode)]
fn main() {
    nekor::entry();
}

#[cfg(not(any(usermode, test)))]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    nekor::entry()
}

#[cfg(not(usermode))]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    let _ = info;

    loop {
        core::hint::spin_loop();
    }
}
