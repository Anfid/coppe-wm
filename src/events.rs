use x11rb::{errors::ReplyError, protocol::Event};

use crate::bindings::Key;
use crate::X11Conn;

pub mod id {
    pub const KEY_PRESS: i32 = 1;
    pub const KEY_RELEASE: i32 = 2;
}

#[derive(Debug)]
pub enum WmEvent {
    KeyPressed(Key),
    KeyReleased(Key),
}

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

    pub fn id(&self) -> i32 {
        use WmEvent::*;
        match self {
            KeyPressed(_) => id::KEY_PRESS,
            KeyReleased(_) => id::KEY_RELEASE,
        }
    }

    pub fn matches(&self, filters: &SubscriptionFilterGroup) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct EncodedEvent(Vec<i32>);

impl EncodedEvent {
    pub fn size(&self) -> usize {
        self.0.len()
    }
}

impl From<&WmEvent> for EncodedEvent {
    fn from(event: &WmEvent) -> Self {
        use WmEvent::*;

        let mut encoded = Vec::new();

        encoded.push(event.id());
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

#[derive(Debug, PartialEq, Eq)]
pub struct Subscription {
    pub event: SubscriptionEvent,
    pub filters: SubscriptionFilterGroup,
}

impl Subscription {
    // TODO: Should be result?
    pub fn parse(buffer: &[i32]) -> Option<Self> {
        match buffer {
            [id::KEY_PRESS, modmask, keycode, filters @ ..] => {
                let event = KeySubscription::new(*modmask as u16, *keycode as u8);
                let filters = SubscriptionFilterGroup::parse(filters)?;
                Some(Self {
                    event: SubscriptionEvent::KeyPressed(event),
                    filters,
                })
            }
            [id::KEY_RELEASE, modmask, keycode, filters @ ..] => {
                let event = KeySubscription::new(*modmask as u16, *keycode as u8);
                let filters = SubscriptionFilterGroup::parse(filters)?;
                Some(Self {
                    event: SubscriptionEvent::KeyReleased(event),
                    filters,
                })
            }
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SubscriptionEvent {
    KeyPressed(KeySubscription),
    KeyReleased(KeySubscription),
}

impl From<&WmEvent> for SubscriptionEvent {
    fn from(ev: &WmEvent) -> Self {
        use SubscriptionEvent::*;
        match ev {
            WmEvent::KeyPressed(key) => {
                KeyPressed(KeySubscription::new(key.modmask.into(), key.keycode))
            }
            WmEvent::KeyReleased(key) => {
                KeyReleased(KeySubscription::new(key.modmask.into(), key.keycode))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SubscriptionFilterGroup(Vec<SubscriptionFilter>);

impl SubscriptionFilterGroup {
    // TODO: Should be result?
    fn parse(buffer: &[i32]) -> Option<Self> {
        Some(Self(Vec::new()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SubscriptionFilter {}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct KeySubscription {
    modmask: u16,
    keycode: u8,
}

impl KeySubscription {
    fn new(modmask: u16, keycode: u8) -> Self {
        KeySubscription { modmask, keycode }
    }
}
