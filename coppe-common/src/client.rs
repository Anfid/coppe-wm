use crate::encoding::*;
use core::convert::TryInto;

#[derive(Debug, Clone, Copy)]
pub struct Client {
    pub id: u32,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Encode for Client {
    type Error = EncodeError;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        if buffer.len() < 12 {
            return Err(EncodeError::BufferSize);
        }

        buffer[0..4].copy_from_slice(&u32::from(self.id).to_le_bytes());
        buffer[4..6].copy_from_slice(&i16::from(self.x).to_le_bytes());
        buffer[6..8].copy_from_slice(&i16::from(self.y).to_le_bytes());
        buffer[8..10].copy_from_slice(&u16::from(self.width).to_le_bytes());
        buffer[10..12].copy_from_slice(&u16::from(self.height).to_le_bytes());

        Ok(())
    }

    fn encoded_size(&self) -> usize {
        12
    }
}

impl Decode for Client {
    type Error = DecodeError;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() < 12 {
            return Err(DecodeError::BadFormat);
        }

        Ok(Self {
            id: u32::from_le_bytes(buffer[0..4].try_into().unwrap()),
            x: i16::from_le_bytes(buffer[4..6].try_into().unwrap()),
            y: i16::from_le_bytes(buffer[6..8].try_into().unwrap()),
            width: u16::from_le_bytes(buffer[8..10].try_into().unwrap()),
            height: u16::from_le_bytes(buffer[10..12].try_into().unwrap()),
        })
    }
}
