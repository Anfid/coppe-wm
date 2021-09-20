pub trait Encode: Sized {
    type Error;

    fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error>;
    fn encoded_size(&self) -> usize;
}

pub trait Decode: Sized {
    type Error;

    fn decode(buffer: &[u8]) -> Result<Self, Self::Error>;
}

#[derive(Debug)]
pub enum EncodeError {
    BufferSize,
}

#[derive(Debug)]
pub enum DecodeError {
    BadFormat,
}

#[cfg(feature = "std")]
pub trait EncodeExt {
    type Error;
    fn encode_to_vec(&self) -> Result<Vec<u8>, Self::Error>;
}

#[cfg(feature = "std")]
impl<E: Encode> EncodeExt for E {
    type Error = E::Error;

    fn encode_to_vec(&self) -> Result<Vec<u8>, Self::Error> {
        let mut buffer = vec![0; self.encoded_size()];
        self.encode_to(buffer.as_mut_slice()).map(|_| buffer)
    }
}
