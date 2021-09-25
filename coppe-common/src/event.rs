use crate::client::ClientId;
use crate::encoding::{Decode, DecodeError, Encode, EncodeError};
use crate::key::Key;

pub mod id {
    pub const KEY_PRESS: u32 = 1;
    pub const KEY_RELEASE: u32 = 2;
    pub const CLIENT_ADD: u32 = 3;
    pub const CLIENT_REMOVE: u32 = 4;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    KeyPress(Key),
    KeyRelease(Key),
    ClientAdd(ClientId),
    ClientRemove(ClientId),
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
            ClientAdd(_) => id::CLIENT_ADD,
            ClientRemove(_) => id::CLIENT_REMOVE,
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
            id::KEY_PRESS => Key::decode(&buffer[4..]).map(Event::KeyPress),
            id::KEY_RELEASE => Key::decode(&buffer[4..]).map(Event::KeyRelease),
            id::CLIENT_ADD => ClientId::decode(&buffer[4..]).map(Event::ClientAdd),
            id::CLIENT_REMOVE => ClientId::decode(&buffer[4..]).map(Event::ClientRemove),
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
            Self::ClientAdd(client) | Self::ClientRemove(client) => {
                client.encode_to(&mut buffer[4..])
            }
        }
    }

    fn encoded_size(&self) -> usize {
        match self {
            Self::KeyPress(key) | Self::KeyRelease(key) => 4 + key.encoded_size(),
            Self::ClientAdd(client) | Self::ClientRemove(client) => 4 + client.encoded_size(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionEvent {
    KeyPress(Key),
    KeyRelease(Key),
    ClientAdd,
    ClientRemove,
}

impl SubscriptionEvent {
    pub fn id(&self) -> u32 {
        use SubscriptionEvent::*;
        match self {
            KeyPress(_) => id::KEY_PRESS,
            KeyRelease(_) => id::KEY_RELEASE,
            ClientAdd => id::CLIENT_ADD,
            ClientRemove => id::CLIENT_REMOVE,
        }
    }
}

impl From<&Event> for SubscriptionEvent {
    fn from(event: &Event) -> Self {
        match event {
            Event::KeyPress(key) => SubscriptionEvent::KeyPress(*key),
            Event::KeyRelease(key) => SubscriptionEvent::KeyRelease(*key),
            Event::ClientAdd(_) => SubscriptionEvent::ClientAdd,
            Event::ClientRemove(_) => SubscriptionEvent::ClientRemove,
        }
    }
}

impl Encode for SubscriptionEvent {
    type Error = EncodeError;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        use SubscriptionEvent::*;

        self.id().encode_to(&mut buffer[0..])?;

        match self {
            KeyPress(key) | KeyRelease(key) => key.encode_to(&mut buffer[4..])?,
            ClientAdd | ClientRemove => {}
        }

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        use SubscriptionEvent::*;

        match self {
            KeyPress(key) | KeyRelease(key) => 4 + key.encoded_size(),
            ClientAdd | ClientRemove => 4,
        }
    }
}

impl Decode for SubscriptionEvent {
    type Error = DecodeError;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
        use SubscriptionEvent::*;

        let id = u32::decode(&buffer)?;

        match id {
            id::KEY_PRESS => Key::decode(&buffer[4..]).map(KeyPress),
            id::KEY_RELEASE => Key::decode(&buffer[4..]).map(KeyRelease),
            id::CLIENT_ADD => Ok(ClientAdd),
            id::CLIENT_REMOVE => Ok(ClientRemove),
            _ => Err(DecodeError::BadFormat),
        }
    }
}

#[cfg(feature = "std")]
#[derive(Debug, PartialEq, Eq)]
pub struct Subscription {
    pub event: SubscriptionEvent,
    pub filters: Vec<SubscriptionFilter>,
}

#[cfg(feature = "std")]
impl From<SubscriptionEvent> for Subscription {
    fn from(event: SubscriptionEvent) -> Self {
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
        let event = SubscriptionEvent::decode(buffer)?;

        Ok(Self::from(event))
    }
}

#[cfg(feature = "std")]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SubscriptionFilter {}
