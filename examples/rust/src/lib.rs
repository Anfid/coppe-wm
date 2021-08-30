#![no_std]

use core::panic::PanicInfo;

pub const MOD_SHIFT: i32 = 1 << 0;
pub const MOD_LOCK: i32 = 1 << 1;
pub const MOD_CONTROL: i32 = 1 << 2;
pub const MOD_M1: i32 = 1 << 3;
pub const MOD_M2: i32 = 1 << 4;
pub const MOD_M3: i32 = 1 << 5;
pub const MOD_M4: i32 = 1 << 6;
pub const MOD_M5: i32 = 1 << 7;
pub const MOD_ANY: i32 = 1 << 15;

extern "C" {
    fn subscribe(event_ptr: *const i32, event_len: usize);
    fn spawn(cmd_ptr: *const u8, cmd_len: usize);
    fn event_read(buf_ptr: *const i32, buf_len: usize, offset: usize) -> isize;
}

const EVENT_KEY_PRESS_ID: i32 = 1;

#[no_mangle]
pub static id: [u8; 10] = *b"rust_demo\0";

#[no_mangle]
pub extern "C" fn init() {
    let subscription = [EVENT_KEY_PRESS_ID, MOD_M4, 36];
    unsafe { subscribe(subscription.as_ptr(), subscription.len()) };
}

#[no_mangle]
pub extern "C" fn handle() {
    let terminal = "kitty -1";

    let mut event = [0; 3];
    unsafe { event_read(event.as_mut_ptr(), event.len(), 0) };

    match event {
        [EVENT_KEY_PRESS_ID, MOD_M4, 36] => unsafe { spawn(terminal.as_ptr(), terminal.len()) },
        _ => {}
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe { core::arch::wasm32::unreachable() }
}
