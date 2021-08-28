#![no_std]

use core::panic::PanicInfo;

extern "C" {
    fn subscribe(event_id: i32);
    fn spawn(cmd_ptr: *const u8, cmd_len: usize);
    fn event_read(buf_ptr: *const i32, buf_len: usize, offset: usize) -> isize;
}

const EVENT_KEY_PRESS_ID: i32 = 1;

#[no_mangle]
pub static id: [u8; 10] = *b"rust_demo\0";

#[no_mangle]
pub extern "C" fn init() {
    unsafe { subscribe(EVENT_KEY_PRESS_ID) }
}

#[no_mangle]
pub extern "C" fn handle() {
    let command = "kitty";

    let mut event = [0; 3];
    let _dwords_read = unsafe { event_read(event.as_mut_ptr(), event.len(), 0) };
    if event[0] == EVENT_KEY_PRESS_ID && event[2] == 38 {
        unsafe { spawn(command.as_ptr(), command.len()) }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe { core::arch::wasm32::unreachable() }
}
