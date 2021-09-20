use coppe_core::ffi;

pub use coppe_core::command::move_window;

pub fn spawn<C: AsRef<str>>(command: C) {
    ffi::spawn(command.as_ref());
}
