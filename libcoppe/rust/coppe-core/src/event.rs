use crate::ffi;
use crate::key::{Keycode, ModMask};
use coppe_common::{
    encoding::{Decode, Encode, EncodeError},
    event::Event as CommonEvent,
};

pub use coppe_common::event::{id, SubscriptionEvent};
pub use coppe_common::window::{Geometry, Window, WindowId};

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

    pub fn from_raw_buffer(buffer: &'a [u8]) -> Self {
        Self { buffer }
    }
}

pub trait SubscriptionEventExt {
    fn init_without_filters(self, buffer: &mut [u8]) -> Result<Subscription, EncodeError>;
}

impl SubscriptionEventExt for SubscriptionEvent {
    fn init_without_filters(self, buffer: &mut [u8]) -> Result<Subscription, EncodeError> {
        self.encode_to(buffer)?;

        Ok(Subscription { buffer })
    }
}

pub enum Event {
    KeyPress(ModMask, Keycode),
    KeyRelease(ModMask, Keycode),
    WindowAdd(WindowId),
    WindowRemove(WindowId),
    WindowConfigure(WindowId, Geometry),
}

impl From<CommonEvent> for Event {
    fn from(event: CommonEvent) -> Self {
        match event {
            CommonEvent::KeyPress(key) => Event::KeyPress(key.modmask, key.keycode),
            CommonEvent::KeyRelease(key) => Event::KeyRelease(key.modmask, key.keycode),
            CommonEvent::WindowAdd(window) => Event::WindowAdd(window),
            CommonEvent::WindowRemove(window) => Event::WindowRemove(window),
            CommonEvent::WindowConfigure(window) => {
                Event::WindowConfigure(window.id, window.geometry)
            }
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
