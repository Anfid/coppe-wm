use crate::encoding::*;

pub type WindowId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Window {
    pub id: WindowId,
    pub geometry: Geometry,
}

impl Encode for Window {
    type Error = EncodeError;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.id.encode_to(&mut buffer[0..])?;
        self.geometry
            .encode_to(&mut buffer[self.id.encoded_size()..])
    }

    fn encoded_size(&self) -> usize {
        self.id.encoded_size() + self.geometry.encoded_size()
    }
}

impl Decode for Window {
    type Error = DecodeError;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
        let id = WindowId::decode(&buffer[0..])?;
        let geometry = Geometry::decode(&buffer[id.encoded_size()..])?;
        Ok(Self { id, geometry })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Geometry {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Encode for Geometry {
    type Error = EncodeError;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.x.encode_to(&mut buffer[0..])?;
        self.y.encode_to(&mut buffer[2..])?;
        self.width.encode_to(&mut buffer[4..])?;
        self.height.encode_to(&mut buffer[6..])
    }

    fn encoded_size(&self) -> usize {
        8
    }
}

impl Decode for Geometry {
    type Error = DecodeError;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            x: i16::decode(&buffer[0..])?,
            y: i16::decode(&buffer[2..])?,
            width: u16::decode(&buffer[4..])?,
            height: u16::decode(&buffer[6..])?,
        })
    }
}
