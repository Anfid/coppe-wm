use crate::ffi;
use crate::key::{Key, Keycode, ModMask};

pub struct Subscription<'a> {
    buffer: &'a [i32],
    len: usize,
}

impl<'a> Subscription<'a> {
    pub fn subscribe(self) {
        subscribe(self)
    }
}

pub enum SubscriptionEvent {
    KeyPress(Key),
    KeyRelease(Key),
}

impl SubscriptionEvent {
    pub fn init_without_filters(
        self,
        buffer: &mut [i32],
    ) -> Result<Subscription, SubscriptionInitError> {
        use SubscriptionEvent::*;

        if buffer.len() < 1 {
            return Err(SubscriptionInitError::BufferTooSmall);
        }
        buffer[0] = self.id();

        let len = match self {
            KeyPress(key) | KeyRelease(key) => {
                if buffer.len() < 3 {
                    return Err(SubscriptionInitError::BufferTooSmall);
                }
                buffer[1] = key.modmask.into();
                buffer[2] = key.keycode.into();
                3
            }
        };

        Ok(Subscription { buffer, len })
    }

    fn id(&self) -> i32 {
        use SubscriptionEvent::*;

        match self {
            KeyPress(_) => ffi::EVENT_KEY_PRESS_ID,
            KeyRelease(_) => ffi::EVENT_KEY_RELEASE_ID,
        }
    }
}

#[derive(Debug)]
pub enum SubscriptionInitError {
    BufferTooSmall,
}

pub enum Event {
    KeyPress(ModMask, Keycode),
    KeyRelease(ModMask, Keycode),
}

pub fn subscribe(sub: Subscription) {
    ffi::subscribe(&sub.buffer[..sub.len]);
}

pub fn unsubscribe(sub: Subscription) {
    ffi::unsubscribe(&sub.buffer[..sub.len])
}

pub fn read_parse() -> Option<Event> {
    let mut buffer = [0; 16];
    read(&mut buffer);
    match buffer {
        [ffi::EVENT_KEY_PRESS_ID, modmask, keycode, ..] => {
            Some(Event::KeyPress(modmask.into(), keycode.into()))
        }
        [ffi::EVENT_KEY_RELEASE_ID, modmask, keycode, ..] => {
            Some(Event::KeyRelease(modmask.into(), keycode.into()))
        }
        _ => None,
    }
}

pub fn read(buffer: &mut [i32]) -> isize {
    ffi::event_read(buffer, 0)
}

pub fn read_from(buffer: &mut [i32], offset: usize) -> isize {
    ffi::event_read(buffer, offset)
}

pub fn size() -> usize {
    ffi::event_len()
}
