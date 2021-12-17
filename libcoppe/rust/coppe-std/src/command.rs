use coppe_core::ffi;

pub fn spawn<C: AsRef<str>>(command: C) {
    ffi::spawn(command.as_ref());
}
