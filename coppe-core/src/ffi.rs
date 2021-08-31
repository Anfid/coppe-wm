mod raw {
    extern "C" {
        // Events
        pub fn subscribe(event_ptr: *const i32, event_len: usize);
        pub fn unsubscribe(event_ptr: *const i32, event_len: usize);
        pub fn event_read(buf_ptr: *mut i32, buf_len: usize, offset: usize) -> isize;
        pub fn event_size() -> usize;

        // Commands
        pub fn spawn(cmd_ptr: *const u8, cmd_len: usize);
        pub fn move_window(id: i32, x: i32, y: i32);

        // Debugging utilities
        pub fn debug_log(cmd_ptr: *const u8, cmd_len: usize);
    }
}

pub const EVENT_KEY_PRESS_ID: i32 = 1;
pub const EVENT_KEY_RELEASE_ID: i32 = 2;

pub fn subscribe(event: &[i32]) {
    unsafe { raw::subscribe(event.as_ptr(), event.len()) }
}

pub fn unsubscribe(event: &[i32]) {
    unsafe { raw::unsubscribe(event.as_ptr(), event.len()) }
}

pub fn event_read(buffer: &mut [i32], offset: usize) -> isize {
    unsafe { raw::event_read(buffer.as_mut_ptr(), buffer.len(), offset) }
}

pub fn event_len() -> usize {
    unsafe { raw::event_size() }
}

pub fn spawn(command: &str) {
    unsafe { raw::spawn(command.as_ptr() as *const u8, command.len()) }
}

pub fn move_window(id: i32, x: i32, y: i32) {
    unsafe { raw::move_window(id, x, y) }
}

pub fn debug_log(message: &str) {
    unsafe { raw::debug_log(message.as_ptr() as *const u8, message.len()) }
}
