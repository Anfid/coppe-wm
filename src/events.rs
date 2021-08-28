use crate::bindings::Key;
use x11rb::protocol::Event;

#[derive(Debug)]
pub enum WmEvent {
    KeyPressed(Key),
    KeyReleased(Key),
}

#[derive(Debug, Clone)]
pub struct EncodedEvent(Vec<i32>);

impl WmEvent {
    pub fn try_from(x_event: &Event) -> Option<Self> {
        match x_event {
            Event::KeyPress(event) => Self::KeyPressed(Key {
                modmask: event.state.into(),
                keycode: event.detail,
            })
            .into(),
            Event::KeyRelease(event) => Self::KeyReleased(Key {
                modmask: event.state.into(),
                keycode: event.detail,
            })
            .into(),
            _ => None,
        }
    }

    pub fn id(&self) -> u32 {
        use WmEvent::*;
        match self {
            KeyPressed(_) => 1,
            KeyReleased(_) => 2,
        }
    }
}

impl EncodedEvent {
    pub fn size(&self) -> usize {
        self.0.len()
    }
}

impl From<&WmEvent> for EncodedEvent {
    fn from(event: &WmEvent) -> Self {
        use WmEvent::*;

        let mut encoded = Vec::new();

        encoded.push(event.id() as i32);
        match event {
            KeyPressed(key) => {
                encoded.push(u16::from(key.modmask) as i32);
                encoded.push(key.keycode as i32);
            }
            KeyReleased(key) => {
                encoded.push(u16::from(key.modmask) as i32);
                encoded.push(key.keycode as i32);
            }
        }

        Self(encoded)
    }
}

impl<I> std::ops::Index<I> for EncodedEvent
where
    I: std::slice::SliceIndex<[i32]>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &I::Output {
        &self.0[index]
    }
}
