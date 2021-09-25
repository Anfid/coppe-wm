use coppe_common::encoding::{Decode, EncodeExt};
use coppe_core::ffi;

pub use coppe_common::event::Subscription;
pub use coppe_core::event::{
    id, len, read as read_to, Event, Subscription as SubscriptionBuffer, SubscriptionEvent,
};

pub trait SubscriptionExt {
    fn subscribe(&self);
    fn unsubscribe(&self);
}

impl SubscriptionExt for Subscription {
    fn subscribe(&self) {
        let buffer = self.event.encode_to_vec().unwrap();

        ffi::subscribe(buffer.as_slice())
    }

    fn unsubscribe(&self) {
        let buffer = self.event.encode_to_vec().unwrap();
        ffi::unsubscribe(buffer.as_slice())
    }
}

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
