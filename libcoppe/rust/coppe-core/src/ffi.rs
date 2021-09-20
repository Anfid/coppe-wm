pub(crate) mod raw {
    extern "C" {
        // Events
        pub fn subscribe(event_ptr: *const u8, event_len: usize) -> i32;
        pub fn unsubscribe(event_ptr: *const u8, event_len: usize) -> i32;
        pub fn event_read(buf_ptr: *mut u8, buf_len: usize, offset: usize) -> isize;
        pub fn event_len() -> usize;

        // State
        pub fn clients_read(buf_ptr: *mut u8, buf_len: usize) -> isize;
        pub fn clients_size() -> usize;

        // Commands
        pub fn spawn(cmd_ptr: *const u8, cmd_len: usize) -> i32;
        pub fn move_window(id: i32, x: i32, y: i32) -> i32;

        // Debugging utilities
        pub fn debug_log(cmd_ptr: *const u8, cmd_len: usize) -> i32;
    }
}

pub fn subscribe(event: &[u8]) {
    unsafe {
        raw::subscribe(event.as_ptr(), event.len());
    }
}

pub fn unsubscribe(event: &[u8]) {
    unsafe {
        raw::unsubscribe(event.as_ptr(), event.len());
    }
}

pub fn event_read(buffer: &mut [u8], offset: usize) -> isize {
    unsafe { raw::event_read(buffer.as_mut_ptr(), buffer.len(), offset) }
}

pub fn event_len() -> usize {
    unsafe { raw::event_len() }
}

pub fn clients_read(buffer: &mut [u8]) -> isize {
    unsafe { raw::clients_read(buffer.as_mut_ptr(), buffer.len()) }
}

pub fn clients_len() -> usize {
    unsafe { raw::clients_size() }
}

pub fn spawn(command: &str) -> i32 {
    unsafe { raw::spawn(command.as_ptr() as *const u8, command.len()) }
}

pub fn move_window(id: i32, x: i32, y: i32) {
    unsafe {
        raw::move_window(id, x, y);
    }
}

pub fn debug_log(message: &str) {
    unsafe {
        raw::debug_log(message.as_ptr() as *const u8, message.len());
    }
}
