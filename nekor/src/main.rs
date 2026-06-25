#![cfg_attr(not(usermode), no_std)]
#![cfg_attr(not(usermode), no_main)]

// NOTE(wip): This is used to drive binary builds to determine whether a build
// would succeed.

use core::panic::PanicInfo;

extern crate nekor_boot;

#[cfg(usermode)]
fn main() {
    todo!()
}

#[unsafe(no_mangle)]
fn _start() -> ! {
    loop {}
}

#[cfg(not(any(usermode, test)))]
#[panic_handler]
pub fn phandle(info: &PanicInfo) -> ! {
    let _ = info;

    loop {}
}
