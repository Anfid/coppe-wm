pub(crate) mod raw {
    extern "C" {
        // Events
        pub fn subscribe(event_ptr: *const u8, event_len: usize) -> i32;
        pub fn unsubscribe(event_ptr: *const u8, event_len: usize) -> i32;
        pub fn event_read(buf_ptr: *mut u8, buf_len: usize, offset: usize) -> isize;
        pub fn event_len() -> usize;

        // Window management
        pub fn window_move(id: i32, x: i32, y: i32) -> i32;
        pub fn window_resize(id: i32, width: u32, height: u32) -> i32;
        pub fn window_move_resize(id: i32, x: i32, y: i32, width: u32, height: u32) -> i32;
        pub fn window_focus(id: i32) -> i32;
        pub fn window_get_properties(
            id: i32,
            x: *mut i32,
            y: *mut i32,
            width: *mut u32,
            height: *mut u32,
        ) -> i32;
        pub fn window_close(id: i32) -> i32;

        // Commands
        pub fn spawn(cmd_ptr: *const u8, cmd_len: usize) -> i32;

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

pub fn spawn(command: &str) -> i32 {
    unsafe { raw::spawn(command.as_ptr() as *const u8, command.len()) }
}

pub fn window_move(id: i32, x: i32, y: i32) {
    unsafe {
        raw::window_move(id, x, y);
    }
}

pub fn window_resize(id: i32, width: u32, height: u32) {
    unsafe {
        raw::window_resize(id, width, height);
    }
}

pub fn window_move_resize(id: i32, x: i32, y: i32, width: u32, height: u32) {
    unsafe {
        raw::window_move_resize(id, x, y, width, height);
    }
}

pub fn window_focus(id: i32) {
    unsafe {
        raw::window_focus(id);
    }
}

pub fn window_get_properties(id: i32, x: &mut i32, y: &mut i32, width: &mut u32, height: &mut u32) {
    unsafe {
        raw::window_get_properties(
            id,
            x as *mut _,
            y as *mut _,
            width as *mut _,
            height as *mut _,
        );
    }
}

pub fn window_close(id: i32) {
    unsafe {
        raw::window_close(id);
    }
}

pub fn debug_log(message: &str) {
    unsafe {
        raw::debug_log(message.as_ptr() as *const u8, message.len());
    }
}
