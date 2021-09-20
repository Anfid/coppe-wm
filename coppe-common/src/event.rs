use crate::encoding::{Decode, DecodeError, Encode, EncodeError};
use crate::key::Key;

pub mod id {
    pub const KEY_PRESS: u32 = 1;
    pub const KEY_RELEASE: u32 = 2;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    KeyPress(Key),
    KeyRelease(Key),
}

impl Event {
    pub fn key_press(key: Key) -> Self {
        Self::KeyPress(key)
    }

    pub fn key_release(key: Key) -> Self {
        Self::KeyRelease(key)
    }

    pub fn id(&self) -> u32 {
        use Event::*;

        match self {
            KeyPress(_) => id::KEY_PRESS,
            KeyRelease(_) => id::KEY_RELEASE,
        }
    }
}

impl Decode for Event {
    type Error = DecodeError;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() < 4 {
            return Err(DecodeError::BadFormat);
        }

        let mut id: [u8; 4] = [0; 4];
        id.copy_from_slice(&buffer[..4]);
        let id = u32::from_le_bytes(id);

        match id {
            id::KEY_PRESS => Key::decode(&buffer[4..]).map(Event::KeyRelease),
            id::KEY_RELEASE => Key::decode(&buffer[4..]).map(Event::KeyPress),
            _ => Err(DecodeError::BadFormat),
        }
    }
}

impl Encode for Event {
    type Error = EncodeError;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        if buffer.len() < self.encoded_size() {
            return Err(EncodeError::BufferSize);
        }

        buffer[0..4].copy_from_slice(&self.id().to_le_bytes());

        match self {
            Self::KeyPress(key) | Self::KeyRelease(key) => key.encode_to(&mut buffer[4..]),
        }
    }

    fn encoded_size(&self) -> usize {
        match self {
            Self::KeyPress(key) | Self::KeyRelease(key) => 4 + key.encoded_size(),
        }
    }
}

#[cfg(feature = "std")]
#[derive(Debug, PartialEq, Eq)]
pub struct Subscription {
    pub event: Event,
    pub filters: Vec<SubscriptionFilter>,
}

#[cfg(feature = "std")]
impl From<Event> for Subscription {
    fn from(event: Event) -> Self {
        Self {
            event,
            filters: vec![],
        }
    }
}

#[cfg(feature = "std")]
impl Encode for Subscription {
    type Error = EncodeError;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.event.encode_to(buffer)
    }

    fn encoded_size(&self) -> usize {
        self.event.encoded_size()
    }
}

#[cfg(feature = "std")]
impl Decode for Subscription {
    type Error = DecodeError;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
        let event = Event::decode(buffer)?;

        Ok(Self::from(event))
    }
}

#[cfg(feature = "std")]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SubscriptionFilter {}
