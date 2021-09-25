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

macro_rules! impl_encoding_for_num {
    ($a:ty) => {
        impl Encode for $a {
            type Error = EncodeError;

            fn encode_to(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
                let size = core::mem::size_of::<Self>();
                if buffer.len() < size {
                    return Err(EncodeError::BufferSize);
                }
                buffer[0..size].copy_from_slice(&self.to_le_bytes());
                Ok(())
            }

            fn encoded_size(&self) -> usize {
                core::mem::size_of::<Self>()
            }
        }

        impl Decode for $a {
            type Error = DecodeError;

            fn decode(buffer: &[u8]) -> Result<Self, Self::Error> {
                use core::convert::TryInto;

                let size = core::mem::size_of::<Self>();
                let res = <$a>::from_le_bytes(buffer[0..size].try_into().map_err(|_| DecodeError::BadFormat)?);
                Ok(res)
            }
        }
    };
    ($($a:ty), +) => {
        $(
            impl_encoding_for_num! { $a }
        )+
    };
}

impl_encoding_for_num! {u8, u16, u32, u64, i8, i16, i32, i64}
