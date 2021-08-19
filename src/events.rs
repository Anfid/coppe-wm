use crate::bindings::Key;
use x11rb::protocol::Event;

#[derive(Debug)]
pub enum WMEvent {
    KeyPressed(Key),
    KeyReleased(Key),
}

impl WMEvent {
    pub fn try_from(x_event: &Event) -> Option<Self> {
        match x_event {
            Event::KeyPress(event) => Self::KeyPressed(Key {
                modmask: event.state.into(),
                keycode: event.detail,
            })
            .into(),
            Event::KeyRelease(event) => Self::KeyPressed(Key {
                modmask: event.state.into(),
                keycode: event.detail,
            })
            .into(),
            _ => None,
        }
    }
}
