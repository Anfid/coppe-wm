use coppe_common::encoding::{Decode, EncodeExt};
use coppe_core::ffi;

pub use coppe_core::event::{
    id, len, read as read_to, Event, Subscription as SubscriptionBuffer, SubscriptionEvent,
};

pub struct Subscription {
    pub event: SubscriptionEvent,
    pub filters: Vec<SubscriptionFilter>,
}

impl Subscription {
    pub fn subscribe(&self) {
        let buffer = self.event.encode_to_vec().unwrap();

        ffi::subscribe(buffer.as_slice())
    }

    pub fn unsubscribe(&self) {
        let buffer = self.event.encode_to_vec().unwrap();
        ffi::unsubscribe(buffer.as_slice())
    }
}

impl From<SubscriptionEvent> for Subscription {
    fn from(event: SubscriptionEvent) -> Self {
        Self {
            event,
            filters: vec![],
        }
    }
}

pub enum SubscriptionFilter {}

pub fn read() -> Option<Event> {
    let len = len();
    if len == 0 {
        None
    } else {
        let mut buffer = vec![0; len];
        read_to(buffer.as_mut_slice());
        Event::decode(&buffer).ok()
    }
}
