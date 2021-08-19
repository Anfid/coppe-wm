#![no_std]

use core::panic::PanicInfo;

extern "C" {
    fn spawn(cmd_ptr: *const u8, cmd_len: usize);
}

#[no_mangle]
pub static id: [u8; 10] = *b"rust_demo\0";

#[no_mangle]
pub extern "C" fn handle() {
    let command = "kitty";

    unsafe { spawn(command.as_ptr(), command.len()) }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe { core::arch::wasm32::unreachable() }
}
