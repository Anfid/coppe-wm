use coppe_core::ffi::debug_log;

pub fn log<M: AsRef<str>>(message: M) {
    debug_log(message.as_ref())
}
