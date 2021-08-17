#![no_std]

use core::panic::PanicInfo;

extern "C" {
    fn move_window(id: i32, x: i32, y: i32);
}

#[no_mangle]
fn handle() {
    unsafe { move_window(1, 200, 300) };
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe { core::arch::wasm32::unreachable() }
}
