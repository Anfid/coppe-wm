#[derive(Debug)]
pub enum WMEvent {
    KeyPressed(crate::bindings::Key),
    KeyReleased(crate::bindings::Key),
}

#[derive(Debug)]
pub enum RunnerEvent {
    MoveWindow { id: u32, x: i32, y: i32 },
}
