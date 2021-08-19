#![no_std]

use core::panic::PanicInfo;

extern "C" {
    fn spawn(cmd_ptr: *const u8, cmd_len: usize);
}

#[no_mangle]
fn handle() {
    let command = "kitty";

    unsafe { spawn(command.as_ptr(), command.len()) }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe { core::arch::wasm32::unreachable() }
}
