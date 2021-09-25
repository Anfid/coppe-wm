use crate::ffi;
use crate::key::{Key, Keycode, ModMask};
use coppe_common::{
    encoding::{Decode, Encode, EncodeError},
    event::Event as CommonEvent,
};

pub use coppe_common::event::id;

pub struct Subscription<'a> {
    buffer: &'a [u8],
}

impl<'a> Subscription<'a> {
    pub fn subscribe(&self) {
        ffi::subscribe(self.buffer)
    }

    pub fn unsubscribe(&self) {
        ffi::unsubscribe(self.buffer)
    }
}

pub struct SubscriptionEvent(CommonEvent);

impl SubscriptionEvent {
    pub fn key_press(key: Key) -> SubscriptionEvent {
        Self(CommonEvent::KeyPress(key))
    }

    pub fn key_release(key: Key) -> SubscriptionEvent {
        Self(CommonEvent::KeyRelease(key))
    }

    pub fn init_without_filters(self, buffer: &mut [u8]) -> Result<Subscription, EncodeError> {
        self.0.encode_to(buffer)?;

        Ok(Subscription { buffer })
    }

    pub fn from_raw_buffer(buffer: &[u8]) -> Subscription {
        Subscription { buffer }
    }

    pub fn id(&self) -> u32 {
        use CommonEvent::*;

        match self.0 {
            KeyPress(_) => id::KEY_PRESS,
            KeyRelease(_) => id::KEY_RELEASE,
        }
    }
}

impl Encode for SubscriptionEvent {
    type Error = <CommonEvent as Encode>::Error;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.0.encode_to(buffer)
    }

    fn encoded_size(&self) -> usize {
        self.0.encoded_size()
    }
}

pub enum Event {
    KeyPress(ModMask, Keycode),
    KeyRelease(ModMask, Keycode),
}

impl From<CommonEvent> for Event {
    fn from(event: CommonEvent) -> Self {
        match event {
            CommonEvent::KeyPress(key) => Event::KeyPress(key.modmask, key.keycode),
            CommonEvent::KeyRelease(key) => Event::KeyRelease(key.modmask, key.keycode),
        }
    }
}

impl Decode for Event {
    type Error = <CommonEvent as Decode>::Error;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
        CommonEvent::decode(buffer).map(Into::into)
    }
}

pub fn read_parse() -> Option<Event> {
    let mut buffer = [0; 16];
    read(&mut buffer);
    CommonEvent::decode(&buffer).map(Into::into).ok()
}

pub fn read(buffer: &mut [u8]) -> isize {
    ffi::event_read(buffer, 0)
}

pub fn read_from(buffer: &mut [u8], offset: usize) -> isize {
    ffi::event_read(buffer, offset)
}

pub fn len() -> usize {
    ffi::event_len()
}
